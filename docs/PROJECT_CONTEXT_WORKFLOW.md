# Project Context Workflow

This document defines the practical workflow for generating and validating distilled project context artifacts in this repository.

## Goal

The objective is not to keep re-reading the full repository. The objective is to keep a small set of canonical context artifacts fresh enough that agents can start from them.

Current artifacts:

- `.context-pack/PROJECT_CONTEXT.md`
- `.context-pack/PROJECT_CONTEXT.json`
- `.context-pack/memory.md`

## Agent Contract

This is the exact contract that can be embedded in `AGENTS.md` or similar repo instructions:

```md
Before deeper exploration, read `.context-pack/PROJECT_CONTEXT.md`.
If `.context-pack/PROJECT_CONTEXT.md`, `.context-pack/PROJECT_CONTEXT.json`, or `.context-pack/memory.md` is missing, run `context-pack context refresh --cwd .`.
If `.context-pack/memory.md` is older than 7 days and repository activity continued, run `context-pack context refresh --cwd .`.
If you discover a durable repo rule, architectural invariant, or recurring pitfall, update `.context-pack/memory.md`.
Before finishing substantial work, decide whether the context artifacts need a refresh.
```

## What Works Today

Today the repository can already do this:

```sh
context-pack context refresh --cwd .
context-pack context check --cwd .
```

`context-pack context refresh --cwd .` currently:

1. refreshes `.context-pack/memory.md`
2. writes `.context-pack/PROJECT_CONTEXT.md`
3. writes `.context-pack/PROJECT_CONTEXT.json`

`context-pack context check --cwd .` validates that those artifacts exist and contain the expected structural markers.

## Recommended CI Policy

The pragmatic rollout is:

1. CI generates fresh artifacts in a smoke-test job.
2. CI validates those artifacts with `make context-check`.
3. Later, if the team wants stricter enforcement, CI can fail when key files changed but committed artifacts were not refreshed.

Suggested policy levels:

- Level 1: smoke test only
  - run `make refresh-context`
  - run `make context-check`
  - do not require committing artifacts

- Level 2: stale protection
  - fail if `.context-pack/memory.md` is stale and repo activity continued
  - warn if `PROJECT_CONTEXT.*` exists but is older than current source changes

- Level 3: artifact enforcement
  - require committed updates to `PROJECT_CONTEXT.md` and `PROJECT_CONTEXT.json` when high-signal files changed
  - examples of high-signal files: `src/**`, `Cargo.toml`, `README.md`, `AGENTS.md`, workflow files, docs that change onboarding meaningfully

## Future CLI Design

The current workflow is implemented through existing commands plus a Makefile wrapper. If this becomes central to the product, the cleaner UX is to add first-class CLI commands.

Suggested commands:

- `context-pack --refresh-project-context`
  - generate `.context-pack/PROJECT_CONTEXT.md`
  - generate `.context-pack/PROJECT_CONTEXT.json`
  - refresh `.context-pack/memory.md`

- `context-pack --check-project-context`
  - verify required artifacts exist
  - report freshness
  - return non-zero exit on missing or invalid artifacts

- `context-pack --project-context-format <md|json|both>`
  - control which artifacts to generate

- `context-pack --fail-on-stale-memory`
  - useful for CI

- `context-pack --stale-after-days <n>`
  - override default freshness threshold

## Why This Matters

This workflow turns repo understanding into something agents can reuse instead of rediscover.

That is the real product direction:

- extract repo signals
- distill them into canonical artifacts
- validate freshness
- make using those artifacts cheaper than ignoring them
