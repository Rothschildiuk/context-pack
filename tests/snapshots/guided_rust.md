# guided-rust-fixture — context pack

> This fixture represents a small CLI-oriented Rust project used for snapshot testing.

- Languages: rust (1)
- Stack: cargo

## Agent instructions
Read and follow these before editing:
- `AGENTS.md` — agent instructions · 3 lines

## Commands
- build: `cargo build` — cargo default
- test: `cargo test` — cargo default
- lint: `cargo clippy --all-targets -- -D warnings` — cargo default
- format: `cargo fmt --all --check` — cargo default
- run: `make run` — Makefile: cargo run

## Entry points
- `src/main.rs` — cargo bin `guided-rust-fixture` (Cargo.toml) · 3 lines

## Layout
- `src/` — 1 file(s), rust

## Docs
- `README.md` — project overview · 9 lines

## Excerpts
### `AGENTS.md`
```
# Agent Rules

Start with `README.md`, then check `Cargo.toml`, and then inspect `src/main.rs`.
```

### `src/main.rs`
```
    1: fn main()
```

<!-- context-pack <VERSION> · schema 2.0 · 5 files indexed -->
