# OpenViking Integration Plan

This document defines a practical integration plan for using OpenViking with `context-pack`.

## Goal

Enable structured, tiered repository context delivery for AI agents while preserving the current fast CLI and MCP workflows.

## Non-Goals

- Do not fork OpenViking as a first step.
- Do not replace current `markdown`/`json` output formats.
- Do not introduce always-on network sync in the initial release.

## Current Baseline

`context-pack` already exposes MCP tools with a versioned response envelope:

- `get_context`
- `get_changed_context`
- `get_file_excerpt`
- `init_memory`
- `refresh_memory`

Each MCP tool returns `structuredContent.schemaVersion` (`"1.0"`), which should remain stable and backward compatible.

## Proposed Tier Mapping

- `L0` (core, stable): guidance docs, repo summary, project/language detection.
- `L1` (active session): changed-file summaries, active work, targeted excerpts.
- `L2` (deep retrieval): full tree, historical memory, larger supporting modules.

## Delivery Strategy

### Phase 1: Export-Only Integration (Recommended)

Add a deterministic export path without network push:

```bash
context-pack --format viking
```

Requirements:

- Define a strict `viking` JSON schema with `schemaVersion`.
- Preserve existing output formats and MCP behavior.
- Add snapshot tests for schema stability.
- Publish the schema description in `docs/schema/Viking.md` so consumers can verify compliance.

### Phase 2: Optional Push Integration

Add explicit sync to OpenViking:

```bash
context-pack --viking-push viking://resources/<project>/briefing
```

Requirements:

- Optional dependency and feature flag gating.
- Retry/idempotency behavior for push operations.
- Safe defaults for redaction before upload.
- Clear CLI errors for auth/network/protocol failures.

## Compatibility Contract

- Keep MCP `structuredContent.schemaVersion` backward compatible.
- If Viking schema changes, bump version and support transition logic.
- Maintain alias compatibility where possible to avoid breaking existing agents.

## Risks and Mitigations

- Sensitive data leakage:
  - Reuse existing redaction behavior and add upload-time safety checks.
- Performance regressions:
  - Keep push optional; export mode must stay local and fast.
- Ecosystem drift:
  - Avoid hard coupling to vendor-specific behavior in core selection logic.

## Success Metrics

- Lower median tokens per agent task.
- Lower time-to-first-correct-answer in onboarding/review workflows.
- Stable parser behavior across releases (no schema-breaking incidents).

## Open Questions

- Should `--format viking` be represented in MCP `get_context` as `format: "viking"` or as a separate MCP tool?
- Should push be CLI-only first, or also exposed via MCP?
- Which minimum OpenViking capability set is required for initial support?
