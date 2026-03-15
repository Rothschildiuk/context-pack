# AI Agent Guide — context-pack

This guide is for coding agents contributing to **this repository** (`context-pack`). It replaces generic agent guidance with specifics for this codebase.

> [!IMPORTANT]
> **Start here, every session:**
> ```sh
> context-pack --cwd . --format json
> ```
> Then read `CONTRIBUTING.md` for the full contributor workflow.

---

## What this repo is

`context-pack` is a Rust CLI that generates a compact, high-signal briefing of a repository for coding agents. It scans files, git history, dependency manifests, and guidance docs, then ranks and excerpts the most important content within a configurable byte budget.

**Your job as a contributor**: make the tool smarter, faster, or more useful — without breaking the output quality that the snapshot tests verify.

---

## Repository layout

```
src/
├── main.rs            ← Orchestrates the full pipeline (start reading here)
├── cli.rs             ← All CLI flags → AppConfig struct
├── model.rs           ← Core types used across modules
├── select.rs          ← ★ File scoring, ranking, candidate selection (1831 LOC)
├── briefing.rs        ← High-level briefing assembly
├── detect.rs          ← Language and project-type detection
├── walk.rs            ← Directory traversal + tree_summary
├── git.rs             ← Git status, branch, changed files
├── ignore.rs          ← .gitignore + --include/--exclude filtering
├── render_markdown.rs ← Markdown output
├── render_json.rs     ← JSON output
├── mcp.rs             ← MCP server (stdio JSON-RPC)
├── diff.rs            ← Diff mode: compare two JSON snapshots
├── docker_summary.rs  ← Docker/Compose parsing
└── dependency_summary.rs ← Cargo/npm/pyproject dependency extraction
tests/
├── agent_briefing.rs     ← Integration tests: correctness on fixture repos
└── markdown_snapshots.rs ← Snapshot tests: markdown output regression
```

---

## The pipeline (read `main.rs → build_context`)

```
parse_args → AppConfig
    ↓
IgnoreMatcher (gitignore + CLI filters)
    ↓
walk.rs    → tree_summary
git.rs     → changed_files, branch_context
select.rs  → scored candidates → top N files with excerpts   ← core
detect.rs  → repo type, primary languages
briefing.rs → read_these_first, likely_entry_points, repo_summary
    ↓
render_markdown.rs or render_json.rs → stdout
```

---

## How file scoring works

All classification is in `src/select.rs → classify()`. Every file gets a category and a base score:

| Category | Base Score | Example files |
|---|---|---|
| `Instructions` | 1000 | `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` |
| `Overview` | 900 | `README.md`, `llms.txt` |
| `Manifest` | 820 | `Cargo.toml`, `package.json` |
| `Build` | 760 | `Makefile`, `Dockerfile` |
| `ChangedSource` | 740 | any tracked changed `.rs` file |
| `EntryPoint` | 700 | `main.rs`, `index.ts` |
| `Config` | 660 | `.env.example` |
| `SupportingDoc` | 520 | `ARCHITECTURE.md`, `CONTRIBUTING.md` |

Bonuses applied in `score_candidate()`:
- `+40` if at repo root
- `+15` if depth == 1
- `+20` if file is compact (≤ 8 KB)
- `+90/+35` if the file is in the git changed set
- language-aware bonus for top-language source files

Files scoring below **120** are dropped.

---

## Where to make common changes

### Add a new file type to boost
→ `src/select.rs → classify()` — add a branch before the fallthrough `return None`

### Add a new "supporting doc" type
→ `src/select.rs → supporting_doc_reason()` — add a match arm

### Add a new entrypoint filename
→ `src/select.rs → is_entrypoint_file()` — add filename to the match

### Add a new manifest file
→ `src/select.rs → is_manifest()` — add filename

### Add a new CLI flag
→ `src/cli.rs` — add to arg parsing and `AppConfig`
→ `src/model.rs` — add field to `AppConfig` if needed
→ `src/main.rs` or relevant module — wire it up

### Change briefing output structure
→ `src/briefing.rs` for logic, `src/model.rs` for types
→ `src/render_json.rs` and `src/render_markdown.rs` for serialization

---

## Test conventions

```sh
# Run all tests
cargo test

# Refresh markdown snapshots after output changes (REQUIRED if you change render output)
UPDATE_EXPECT=1 cargo test

# Run one test by name
cargo test agent_briefing

# Debug output
cargo test -- --nocapture
```

**When to add a test:**
- New file type heuristic → add fixture test in `tests/agent_briefing.rs`
- New CLI flag → add integration test
- New output field → add assertion in `tests/agent_briefing.rs`

**Do not manually edit snapshot files** in `tests/markdown_snapshots.rs` — always use `UPDATE_EXPECT=1`.

---

## Anti-patterns for this repo

- Editing snapshot files by hand — always regenerate
- Adding broad new file categories without a fixture test
- Changing score constants without running benchmarks on real repos
- Adding a new module without wiring it into `main.rs`
- Assuming the changed-only fast path and the full-scan path behave the same way — they are separate code paths in `select.rs`

---

## Quality bar

A change is ready if:

```sh
cargo test          # all pass
cargo clippy -- -D warnings  # zero warnings  
cargo fmt --check   # clean
cargo run -- --cwd . --format json  # produces valid JSON
```
