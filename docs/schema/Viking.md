# Viking JSON Schema

This document defines the structure emitted by `context-pack --format viking` and the payload exposed via MCP `structuredContent` (see `docs/OPEN_VIKING_INTEGRATION.md`). The schema is intentionally narrow to keep `L0/L1/L2` tiers predictable for OpenViking-style agents.

## Schema Versions

- `schema_version`: hard-coded to `"1.0"` when the CLI output is generated.
- `structuredContent.schemaVersion`: mirrors the same value inside MCP responses for downstream parsers.

Any future schema bump must preserve the old version string and emit a new constant (`"1.1"`, etc.). Tools must safely fall back when they see an unrecognized version.

## Top-level Document

```json
{
  "schema_version": "1.0",
  "format": "viking",
  "tiers": {
    "L0": { ... },
    "L1": { ... },
    "L2": { ... }
  }
}
```

The `tiers` object is the core of the schema. Each level is optional (empty arrays/objects are valid) but agents should assume a tier may be missing and handle it gracefully.

## Tier Details

- **L0 (Core)**
  - `repo.path`: absolute canonicalized repository path.
  - `repo.project_types`: array of strings (e.g., `"rust"`, `"python"`).
  - `repo.primary_languages`: weighted languages detected by `detect::detect_repo_info_with_matcher`.
  - `guidance.repo_summary`: agent-readable sentences.
  - `guidance.read_these_first`: objects `{ "path": <path>, "reason": <why> }`.
  - `guidance.caveats`: array of strings from `briefing.caveats`.

- **L1 (Session)**
  - `active.active_work`: strings describing active git work.
  - `active.likely_entry_points`: same shape as `read_these_first`.
  - `active.git`: includes `available`, `summary`, `branch_context` (`current_branch`, `ahead`, `behind`) and `changes` (entries `{path,status,kind,hint}`).
  - `active.selected_files`: `ImportantFile` details (path, reason, why, category, score, truncated, redacted, redaction_reason, excerpt).

- **L2 (Deep)**
  - `deep.tree_summary`: text output of tree builder, already truncated based on budget.
  - `deep.dependency_summary`: from `dependency_summary::collect`.
  - `deep.docker_summary`: from `docker_summary::collect`.
  - `deep.large_code_files`: objects `{path,loc,reason}`.
  - `deep.notes`: `render_bundle` notes (budget, elapsed, token estimate, etc.)

## MCP Structured Content

The MCP tools wrap the CLI output with `structuredContent`, so agents calling `get_context`/`get_changed_context` can parse the same schema without re-parsing markdown. Example:

```json
{
  "content": [ ... ],
  "structuredContent": {
    "schemaVersion": "1.0",
    "tool": "get_context",
    "status": "ok",
    "data": {
      "cwd": "/Users/...",
      "format": "viking",
      "changedOnly": false,
      "payload": { ... the same tiered document ... }
    }
  }
}
```

Clients can ignore `content` if they prefer the JSON path; the CLI output is preserved for humans.

## Usage Example

1. Run the CLI and pipe it into an OpenViking mount point (assuming `viking://` driver is already configured):

```bash
context-pack --cwd /repo --format viking > /tmp/repo-viking.json
openviking import /tmp/repo-viking.json --target viking://resources/repo/briefing
```

2. Or, from an agent: call the MCP `get_context` tool with `"format": "viking"` and push `structuredContent.data.payload` into the OpenViking API.

## Next Steps

- Document what fields are critical for each tier (e.g., `L0.guidance` should include at least one `read_these_first`).
- Align Phase 2 push workflow with OpenViking ingestion endpoints once the schema stabilizes.
- See `docs/schema/LayeredContext.md` for a draft schema that separates stable context, active context, retrieval targets, and decision memory more explicitly.
