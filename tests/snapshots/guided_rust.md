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
- `src/main.rs`: entrypoint-like source file, language-aware boost (rust, top-1)
- `Makefile`: build or orchestration entrypoint

### Likely Entry Points
- `src/main.rs`: entrypoint-like source file, language-aware boost (rust, top-1)
- `Makefile`: build or orchestration entrypoint

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
- why: project manifest, repo root priority, compact file bonus
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
- why: build or orchestration entrypoint, repo root priority, compact file bonus
- category: build
- score: 820
- truncated: false

```text
.PHONY: run

run:
	cargo run
```

### src/main.rs
- reason: entrypoint-like source file, language-aware boost (rust, top-1)
- why: entrypoint-like source file, shallow path priority, compact file bonus, language-aware boost (rust, top-1)
- category: entrypoint
- score: 805
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
- approx tokens: 649
- elapsed_ms: 1
- max files: 12
- max depth: 4
- budget split: briefing=900, git=500, excerpts=1800, tree=800
- selected files: 5
- language-aware scoring: top languages = rust
- files scanned for selection: 5
