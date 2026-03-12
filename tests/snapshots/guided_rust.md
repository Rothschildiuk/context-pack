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
- `Cargo.toml`: project manifest
- `src/main.rs`: entrypoint-like source file
- `Makefile`: build or orchestration entrypoint

### Likely Entry Points
- `src/main.rs`: entrypoint-like source file
- `Makefile`: build or orchestration entrypoint

### Large Code Files
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
- truncated: false

```text
# Guided Rust Fixture

This fixture represents a small CLI-oriented Rust project used for snapshot testing.

## Workflow

- read the agent instructions
- inspect the manifest
- inspect the entrypoint
```

### Cargo.toml
- reason: project manifest
- category: manifest
- score: 880
- truncated: false

```text
[package]
name = "guided-rust-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
```

### Makefile
- reason: build or orchestration entrypoint
- category: build
- score: 820
- truncated: false

```text
.PHONY: run

run:
	cargo run
```

### src/main.rs
- reason: entrypoint-like source file
- category: entrypoint
- score: 735
- truncated: false

```text
fn main() {
    println!("guided fixture");
}
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
- max bytes: 4000
- max files: 12
- max depth: 4
- budget split: briefing=900, git=500, excerpts=1800, tree=800
- selected files: 5
- files scanned for selection: 5
