# Contributing

## Setup

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- --cwd .            # the tool on itself
cargo run -- --cwd ../some-repo # the tool on anything else
```

Dependencies are deliberately few: `ignore` and `globset` (file walking with real gitignore semantics), `toml`, `serde`, `serde_json`.

## Pipeline

`main.rs` dispatches commands; a briefing is built by `briefing::build`:

1. `index.rs` walks the repository once (gitignore-aware, including nested `.gitignore`) and keeps paths and sizes. Every later stage reads from this index.
2. `paths.rs` classifies each path: language, and role (`source`, `tests`, `examples`, `docs`, `vendor`, ...). Roles keep example and fixture trees from crowding out real code.
3. `manifest.rs` parses manifests (Cargo, npm, pyproject, go.mod, Maven, Gradle, .NET, ...) into names, descriptions, declared entry points, scripts, dependencies, and workspace members.
4. `commands.rs` finds build/test/lint/run commands: Makefile, justfile, Taskfile, package scripts, ecosystem defaults, and the commands CI runs.
5. `git.rs` collects branch state, uncommitted changes, commits on the branch versus its base, recent commits, and per-file change frequency.
6. `select.rs` picks agent instructions, entry points, key source files, docs, and config.
7. `layout.rs` builds the directory map.
8. `memory.rs` reads `.context-pack/memory.md`.
9. `briefing.rs` assembles `model::Brief`, trims it to `--max-bytes`, and fills leftover budget with excerpts (`excerpt.rs`).
10. `render_markdown.rs` / `render_json.rs` render the same `Brief`.

`mcp.rs` exposes the same functionality over MCP (stdio JSON-RPC).

## Changing heuristics

Most improvements are ranking changes in `select.rs`, role rules in `paths.rs`, or new manifest and command sources. The workflow that keeps them honest:

1. Find a public repo where the briefing is wrong (wrong entry point, missing command, noisy section).
2. Reproduce the shape in a test in `tests/agent_briefing.rs` with `TempRepo`: only the files that matter, not a copy of the repo.
3. Fix the heuristic and make sure the other tests still pass.
4. Run the tool on a few real repos of different kinds (a monorepo, a library, an application) and check nothing regressed.

## Output rules

- Every line should change what an agent does next. Internal scores, timing, and budget arithmetic stay out of the markdown.
- The markdown has to fit `--max-bytes`; add new sections to the trimming steps in `briefing::fit_to_budget`.
- JSON mirrors `model::Brief`. Removing or renaming a field requires bumping `model::SCHEMA_VERSION`.
- File contents are redacted before output (`excerpt::redact`), and sensitive files (`.env`, keys) are never excerpted.

## Pull requests

- `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass.
- If markdown output changed on purpose: `UPDATE_EXPECT=1 cargo test --test markdown_snapshots` and include the snapshot diff.
- Add a line to `CHANGELOG.md` under `Unreleased`.

## Releasing

Push a `vX.Y.Z` tag. The release workflow builds macOS and Linux binaries, publishes the npm wrapper, and updates `Formula/context-pack.rb`. Bump `Cargo.toml` first and run `scripts/sync-npm-version.sh`.
