# Layered Context Schema (Draft)

Status: draft only. This document describes a possible future structured output for `context-pack`. It is not a shipped CLI format yet.

## Why This Exists

Large context windows do not solve orientation by themselves. Coding agents work better when the repo briefing is split into layers with different stability and refresh rates.

The design goal is context density:

- stable project rules should not be re-derived every turn
- active task state should not pollute long-lived memory
- retrieval targets should be explicit
- fresh-thread handoff should not require replaying the full chat

## Proposed Top-level Shape

```json
{
  "schema_version": "0.1-draft",
  "format": "layered_context",
  "layers": {
    "stable_context": { ... },
    "active_context": { ... },
    "retrieval_context": { ... },
    "decision_log": { ... },
    "freshness": { ... }
  }
}
```

## Layer Definitions

### `stable_context`

Repository facts that should remain useful across many threads:

- repo path
- project types
- primary languages
- guidance docs (`AGENTS.md`, `README.md`, `llms.txt`, `.clio/instructions.md`)
- long-lived repo memory excerpts
- core entrypoints and build files
- stable caveats (for example: no README, no git, placeholder-heavy docs)

Example:

```json
{
  "repo": {
    "path": "/repo",
    "project_types": ["rust", "node"],
    "primary_languages": ["rust", "javascript"]
  },
  "guidance": {
    "repo_summary": ["Likely a Rust CLI or developer tooling project."],
    "read_these_first": [
      { "path": "AGENTS.md", "reason": "agent instructions" },
      { "path": "README.md", "reason": "project overview" }
    ]
  }
}
```

### `active_context`

Short-lived signals tied to the current branch or task:

- active git work
- current branch context
- changed files
- likely entrypoints for the active work
- current dependency or runtime notes that matter right now
- optional current errors or failing test summaries in a future extension

Example:

```json
{
  "active_work": ["M `src/main.rs` (modified, +12 -4)"],
  "branch": {
    "current_branch": "feature/layered-context",
    "comparison_target": "origin/main"
  },
  "likely_entry_points": [
    { "path": "src/main.rs", "reason": "changed source file, likely entry point" }
  ]
}
```

### `retrieval_context`

Pointers for what the agent should fetch next instead of injecting the full repository:

- ranked files to retrieve next
- why each file matters
- suggested depth or excerpt policy
- optional neighboring files or related tests

Example:

```json
{
  "next_files": [
    {
      "path": "src/select.rs",
      "reason": "core ranking logic",
      "why": ["changed source", "referenced by active work"]
    }
  ]
}
```

### `decision_log`

Compact task history meant for handoff and fresh-thread reuse, not full chat replay:

- accepted design choices
- rejected approaches when they explain current structure
- operational notes worth preserving
- open questions

This layer should stay short and aggressively deduplicated.

Example:

```json
{
  "recent_decisions": [
    "Prefer context density over larger prompt budgets.",
    "Warn when repo memory is stale rather than auto-refreshing every run."
  ],
  "open_questions": [
    "Should layered output become a first-class CLI format or stay MCP-only?"
  ]
}
```

### `freshness`

Metadata that tells the agent whether supporting memory should still be trusted:

- memory creation timestamp
- memory refresh timestamp
- stale flag
- stale reason
- optional source of freshness (`metadata`, `filesystem`, `git`)

Example:

```json
{
  "repo_memory": {
    "created_at_utc": "2026-03-19T22:10:00Z",
    "refreshed_at_utc": "2026-03-25T09:15:00Z",
    "stale": false,
    "stale_reason": null
  }
}
```

## Relation to Current Formats

- Markdown: already useful for human copy/paste, but mixes stable and active signals into one stream.
- JSON (`schema_version: 1.1`): machine-friendly, but not yet explicitly layered.
- Viking (`L0`/`L1`/`L2`): tiered by abstraction depth rather than by memory role.

This draft is intentionally different: the split is based on memory behavior (`stable`, `active`, `retrieval`, `decision`, `freshness`), not just output depth.

## Non-goals

- becoming a full persistent memory runtime
- storing the full chat transcript
- replacing semantic search or RAG
- auto-summarizing every turn without freshness controls

## Rollout Idea

1. Keep current formats stable.
2. Add layered fields behind a draft CLI or MCP flag.
3. Validate on fresh-thread and handoff workflows.
4. Only promote the schema once it improves task outcomes, not just aesthetics.
