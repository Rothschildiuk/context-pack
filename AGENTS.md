# AGENTS.md

`context-pack` is a Rust CLI and MCP server that produces a first-pass briefing of a repository for coding agents. It tells the agent which instructions to follow, which commands build and test the project, where the entry points are, which files matter, and what is currently changing.

## Before you edit

- Run the tool on itself: `cargo run -- --cwd .` (or `context-pack` if installed). It is the fastest map of this repo.
- Read `CONTRIBUTING.md` for the pipeline and the rules for adding heuristics.
- Durable notes about this repo live in `.context-pack/memory.md` (gitignored, local). Add a fact with `context-pack memory add "<fact>"` when you learn something non-obvious.

## Commands

- Build: `cargo build`
- Test: `cargo test` (unit tests in `src/`, end-to-end tests in `tests/agent_briefing.rs`, golden files in `tests/snapshots/`)
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --check`
- Refresh golden files after an intended output change: `UPDATE_EXPECT=1 cargo test --test markdown_snapshots`, then review the diff.

All four checks must pass before a change is done. CI runs the same commands.

## Rules

- The output is read by agents. Every line must help an agent act: no scores, no timing, no marketing, no instructions aimed at the reader's behaviour.
- A heuristic change needs an end-to-end test in `tests/agent_briefing.rs` that reproduces the repo shape it fixes (see the existing tests for fastapi-, astro-, and maven-style layouts).
- Keep the markdown within `--max-bytes`. New sections must be trimmable in `briefing::fit_to_budget`.
- JSON is serialized from `model::Brief`. Adding a field is fine; renaming or removing one requires bumping `SCHEMA_VERSION`.
- Never read outside the target repository (see `mcp::resolve_inside`), and redact secrets before emitting file contents (`excerpt::redact`).
- `.context-pack/memory.md` belongs to its authors. The tool may only rewrite the metadata block.
