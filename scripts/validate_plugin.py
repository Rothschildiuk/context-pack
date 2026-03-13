#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    plugin_manifest = load_json(REPO_ROOT / ".codex-plugin" / "plugin.json")
    mcp_manifest = load_json(REPO_ROOT / ".mcp.json")

    validate_plugin_manifest(plugin_manifest)
    validate_mcp_manifest(mcp_manifest)
    validate_openai_yaml(REPO_ROOT / "skills" / "context-pack" / "agents" / "openai.yaml")
    smoke_test_mcp()

    print("plugin-check: OK")
    return 0


def load_json(path: Path) -> dict:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError as exc:
        fail(f"missing file: {path.relative_to(REPO_ROOT)}", exc)
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON in {path.relative_to(REPO_ROOT)}", exc)


def validate_plugin_manifest(plugin_manifest: dict) -> None:
    for field in ["name", "version", "description", "skills", "mcpServers", "interface"]:
        if field not in plugin_manifest:
            fail(f"plugin manifest missing required field '{field}'")

    for path_field in ["skills", "mcpServers"]:
        relative = plugin_manifest[path_field]
        if not isinstance(relative, str):
            fail(f"plugin manifest field '{path_field}' must be a string")
        assert_exists(relative, f"plugin manifest path '{path_field}'")

    interface = plugin_manifest["interface"]
    if not isinstance(interface, dict):
        fail("plugin manifest field 'interface' must be an object")

    for field in ["displayName", "shortDescription", "category", "websiteURL", "defaultPrompt"]:
        if not interface.get(field):
            fail(f"plugin interface missing required field '{field}'")

    for asset_field in ["composerIcon", "logo"]:
        if asset_field in interface:
            assert_exists(interface[asset_field], f"plugin interface asset '{asset_field}'")

    print("plugin-check: plugin.json looks valid")


def validate_mcp_manifest(mcp_manifest: dict) -> None:
    servers = mcp_manifest.get("mcpServers")
    if not isinstance(servers, dict):
        fail(".mcp.json must contain an object field 'mcpServers'")

    context_pack = servers.get("context-pack")
    if not isinstance(context_pack, dict):
        fail(".mcp.json must define mcpServers.context-pack")

    if context_pack.get("type") != "stdio":
        fail("mcpServers.context-pack.type must be 'stdio'")
    if context_pack.get("command") != "context-pack":
        fail("mcpServers.context-pack.command must be 'context-pack'")
    if "--mcp-server" not in context_pack.get("args", []):
        fail("mcpServers.context-pack.args must include '--mcp-server'")

    print("plugin-check: .mcp.json looks valid")


def validate_openai_yaml(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    for marker in ["display_name:", "short_description:", "default_prompt:"]:
        if marker not in text:
            fail(f"{path.relative_to(REPO_ROOT)} missing '{marker}'")

    for key in ["icon_small", "icon_large"]:
        token = f'{key}: "'
        start = text.find(token)
        if start == -1:
            continue
        end = text.find('"', start + len(token))
        if end == -1:
            fail(f"{path.relative_to(REPO_ROOT)} has unterminated value for {key}")
        relative = text[start + len(token):end]
        skill_root = path.parent.parent
        asset_path = skill_root / relative[2:] if relative.startswith("./") else skill_root / relative
        assert_exists(asset_path, f"skill asset '{key}'")

    print("plugin-check: openai.yaml references look valid")


def smoke_test_mcp() -> None:
    command = ["cargo", "run", "--quiet", "--", "--mcp-server"]
    requests = [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "plugin-check", "version": "1.0.0"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "brief_repo",
                "arguments": {
                    "cwd": str(REPO_ROOT),
                    "noGit": True,
                    "noTree": True,
                    "maxBytes": 1400,
                },
            },
        },
    ]

    payload = "\n".join(json.dumps(item) for item in requests) + "\n"
    result = subprocess.run(
        command,
        cwd=REPO_ROOT,
        input=payload,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        fail("MCP smoke test failed", result.stderr.strip() or result.stdout.strip())

    responses = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    if len(responses) != 3:
        fail(f"expected 3 MCP responses, got {len(responses)}")

    initialize = response_by_id(responses, 1)
    tools_list = response_by_id(responses, 2)
    brief_repo = response_by_id(responses, 3)

    if initialize.get("result", {}).get("serverInfo", {}).get("name") != "context-pack":
        fail("initialize response did not advertise the context-pack server")

    tool_names = {tool["name"] for tool in tools_list.get("result", {}).get("tools", [])}
    expected = {"brief_repo", "init_memory", "refresh_memory"}
    if expected - tool_names:
        fail(f"tools/list missing expected tools: {', '.join(sorted(expected - tool_names))}")

    content = brief_repo.get("result", {}).get("content", [])
    if not content or "# Context Pack" not in content[0].get("text", ""):
        fail("brief_repo did not return the expected briefing text")

    print("plugin-check: MCP smoke test passed")


def response_by_id(responses: list[dict], expected_id: int) -> dict:
    for response in responses:
        if response.get("id") == expected_id:
            return response
    fail(f"missing MCP response id={expected_id}")


def assert_exists(relative_or_path: str | Path, description: str) -> None:
    if isinstance(relative_or_path, Path):
        path = relative_or_path
    elif relative_or_path.startswith("./"):
        path = REPO_ROOT / relative_or_path[2:]
    else:
        path = REPO_ROOT / relative_or_path

    if not path.exists():
        fail(f"{description} does not exist: {path.relative_to(REPO_ROOT)}")


def fail(message: str, detail: object | None = None) -> None:
    print(f"plugin-check: {message}", file=sys.stderr)
    if detail:
        print(detail, file=sys.stderr)
    raise SystemExit(1)


if __name__ == "__main__":
    raise SystemExit(main())
