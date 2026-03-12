# Context Pack

## Agent Briefing
### What This Repo Is
- Likely a Rust project with Cargo-based entry points.
- Primary languages: rust.
- Guidance files available: AGENTS.md, README.

### Active Work
- Git collection disabled

### Read These First
- `AGENTS.md`: agent instructions
- `README.md`: project overview

### Likely Entry Points
- none

### Caveats
- Git collection disabled.

## Repo
- path: <FIXTURE_ROOT>
- project types: rust
- primary languages: rust

## Git Changes
Git collection disabled.

## Important Files
### AGENTS.md
- reason: agent instructions
- category: instructions
- score: 1060
- truncated: false

```text
# Agent Rules

Start with `README.md`, then check `Cargo.toml`, and then inspect `src/main.rs`.
```

### README.md
- reason: project overview
- category: overview
- score: 960
- truncated: true

```text
# Guided Rust Fixture

This fixture represents a small CLI-oriented Rust project used for snapshot testing.

## Workflow

- read the agent instructions
- inspect the manifest
... [truncated]
```

## Tree
guided_rust/
  AGENTS.md
  Cargo.toml
  Makefile
  README.md
  src/
    main.rs

## Notes
- max bytes: 700
- max files: 12
- max depth: 4
- budget split: briefing=260, git=120, excerpts=240, tree=140
- selected files: 2
- files scanned for selection: 5
