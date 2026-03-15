---
name: context-pack
description: Generate a compact first-pass repository briefing with context-pack. Use when work starts in an unfamiliar repo, when the user asks for orientation or high-signal files, when active changes need a compact summary, or when prompt budget matters.
metadata:
  short-description: First-pass repo briefing
---

# Context Pack

Use this skill to turn a repository into a small, prioritized briefing before deeper exploration.

If the `context-pack` MCP server is installed through this plugin, prefer the MCP tools over shelling out manually:

- `get_context`
- `get_changed_context`
- `get_file_excerpt`
- `init_memory`
- `refresh_memory`

## Workflow

1. Start with `context-pack --cwd <repo>` or the `get_context` MCP tool before a manual tree walk unless the task is already extremely narrow.
2. Read the generated briefing first. Prioritize guidance docs, manifests, entrypoints, active work, and caveats before opening large source files.
3. Pick flags based on the task:
   - Active work: `context-pack --cwd <repo> --changed-only --no-tree`
   - Machine-readable output: `context-pack --cwd <repo> --format json`
   - Tight prompt budget: `context-pack --cwd <repo> --changed-only --no-tree --max-bytes 2000`
   - Noisy repo tuning: add `--include`, `--exclude`, `--max-files`, or `--max-depth`
4. If the user wants persistent learned notes, use:
   - `context-pack --cwd <repo> --init-memory`
   - `context-pack --cwd <repo> --refresh-memory`
   - or the matching `init_memory` / `refresh_memory` MCP tools
5. Only after the briefing is in hand should you move to manual file reads, targeted search, or code changes.

## Output Focus

- Name the repo shape and likely entrypoints.
- Call out the most useful files to read next.
- Summarize active work separately from static repo structure.
- Mention caveats such as missing `AGENTS.md`, missing `README`, disabled git, or truncated output.
- Quote the exact `context-pack` command you used when it affects the result.

## Patterns

- Orientation: use the default markdown output and summarize what to read first.
- Reviewing ongoing work: use `--changed-only` to keep the bundle focused on active files and diffs.
- Automation: prefer `--format json` when another tool or script needs the result.
- Legacy repos: initialize or refresh `.context-pack/memory.md` when repo-authored docs are weak.

## Guardrails

- Do not substitute a raw `tree`, blind `rg`, or random large-file reads for the first pass when `context-pack` is available.
- If the generated bundle is too broad or too thin, rerun with tighter budgets or explicit include/exclude globs instead of guessing.
- Treat repo-type summaries and rankings as heuristics; if the code disagrees, say so explicitly.
