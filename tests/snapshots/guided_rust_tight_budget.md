# Context Pack

## Agent Briefing
### What This Repo Is
- Likely a Rust CLI or developer tooling project.
- Primary languages: rust.
- Guidance files available: AGENTS.md, README.

### Active Work
- Git collection disabled

### Read These First
- `AGENTS.md`: agent instructions
- `README.md`: project overview

### Caveats
- Git collection disabled.

## Repo
- path: <FIXTURE_ROOT>
- project types: rust
- primary languages: rust

## Important Files
### AGENTS.md
- reason: agent instructions
- why: agent instructions, repo root priority, compact file bonus
- category: instructions
- score: 1060
- truncated: false

```text
# Agent Rules

Start with `README.md`, then check `Cargo.toml`, and then inspect `src/main.rs`.
```

### README.md
- reason: project overview
- why: project overview, repo root priority, compact file bonus
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
- approx tokens: 364
- max files: 12
- max depth: 4
- budget split: briefing=260, git=120, excerpts=240, tree=140
- selected files: 2
- language-aware scoring: top languages = rust
- files scanned for selection: 5
