# Layered Context Issue Backlog

This file turns the layered-memory direction into concrete implementation tasks for contributors.

## Issue 1: Add a `layered_context` structured output

- Title: `feat: add layered_context structured output`
- Why: current JSON is machine-friendly, but it does not explicitly separate stable context, active context, retrieval targets, and freshness.
- Likely files: `src/render_json.rs`, `src/render_viking.rs`, `src/model.rs`, `src/mcp.rs`, `docs/schema/LayeredContext.md`
- Acceptance criteria:
- add a draft output mode or MCP-only payload for `stable_context`, `active_context`, `retrieval_context`, `decision_log`, and `freshness`
- document the schema version and fallback behavior
- add integration tests for the new structure

## Issue 2: Add task-aware profiles beyond size presets

- Title: `feat: add task-aware profiles for coding and handoff`
- Why: optimal context differs by task, not only by byte budget.
- Likely files: `src/cli.rs`, `src/model.rs`, `src/select.rs`, `src/briefing.rs`, `README.md`
- Acceptance criteria:
- introduce profiles such as `coding`, `debug`, and `handoff`
- ensure each profile changes selection behavior, not just `max_bytes`
- add fixture tests showing profile-specific ranking differences

## Issue 3: Surface retrieval targets explicitly

- Title: `feat: emit explicit retrieval targets for next-file selection`
- Why: agents should know what to open next without injecting the full repo.
- Likely files: `src/briefing.rs`, `src/render_json.rs`, `src/render_viking.rs`, `tests/agent_briefing.rs`
- Acceptance criteria:
- add a small ranked `retrieval_targets` list
- include reason and `why` metadata for each target
- keep the list compact and budget-aware

## Issue 4: Add short decision-log support to repo memory

- Title: `feat: support compact decision_log in .context-pack/memory.md`
- Why: fresh-thread reuse needs short decisions and open questions, not a full transcript.
- Likely files: `src/main.rs`, `src/memory.rs`, `README.md`, `tests/agent_briefing.rs`
- Acceptance criteria:
- extend the generated memory template with a dedicated decision-log section
- preserve human editability
- avoid rewriting user-authored notes during refresh unless the user explicitly opts in

## Issue 5: Expand repo-memory freshness signals

- Title: `feat: improve repo memory freshness scoring and warnings`
- Why: timestamping alone is useful, but agents also need stronger freshness heuristics.
- Likely files: `src/memory.rs`, `src/git.rs`, `src/briefing.rs`, `src/render_json.rs`
- Acceptance criteria:
- combine timestamps with git commit recency and working-tree activity
- expose freshness in structured outputs, not only markdown notes
- distinguish `fresh`, `aging`, and `stale` states

## Issue 6: Build a context-density evaluation harness

- Title: `feat: add context-density evals for fresh-thread workflows`
- Why: smaller output is not enough; the tool should improve downstream behavior.
- Likely files: `tests/`, `README.md`, `CONTEXT_PACK_PLAN.md`, optional `promptfoo` config updates
- Acceptance criteria:
- define at least three before/after evaluation cases
- measure prompt size, first correct file, and wrong-file starts
- document the results in the README once the signal is strong enough

## Issue 7: Add MCP tools for layer-specific retrieval

- Title: `feat: expose stable and active context as separate MCP tools`
- Why: orchestration layers should be able to request only the memory layer they need.
- Likely files: `src/mcp.rs`, `src/main.rs`, `docs/schema/LayeredContext.md`
- Acceptance criteria:
- add tool(s) such as `get_stable_context` and `get_active_context`, or equivalent arguments on existing tools
- keep backward compatibility with `get_context`
- document recommended orchestration patterns
