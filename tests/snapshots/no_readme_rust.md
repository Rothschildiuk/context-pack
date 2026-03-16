# Context Pack

## Agent Briefing
### What This Repo Is
- Likely a Rust CLI or developer tooling project.
- Primary languages: rust.

### Active Work
- Git collection disabled

### Read These First
- `Cargo.toml`: project manifest
- `src/main.rs`: entrypoint-like source file, language-aware boost (rust, top-1)

### Likely Entry Points
- `src/main.rs`: entrypoint-like source file, language-aware boost (rust, top-1)

### Caveats
- No AGENTS.md found.
- No README found.
- Git collection disabled.

## Repo
- path: <FIXTURE_ROOT>
- project types: rust
- primary languages: rust

## Important Files
### Cargo.toml
- reason: project manifest
- why: project manifest, repo root priority, compact file bonus
- category: manifest
- score: 880
- truncated: false

```text
[package]
name = "no-readme-rust-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
```

### src/main.rs
- reason: entrypoint-like source file, language-aware boost (rust, top-1)
- why: entrypoint-like source file, shallow path priority, compact file bonus, language-aware boost (rust, top-1)
- category: entrypoint
- score: 805
- truncated: false

```text
fn main() {
    println!("no readme fixture");
}
```

## Tree
no_readme_rust/
  Cargo.toml
  src/
    main.rs

## Notes
- max bytes: 4000
- approx tokens: 381
- max files: 12
- max depth: 4
- budget split: briefing=900, git=500, excerpts=1800, tree=800
- selected files: 2
- language-aware scoring: top languages = rust
- files scanned for selection: 2
