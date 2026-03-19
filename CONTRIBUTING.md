# Contributing to context-pack

This guide is for both humans and AI coding agents. It tells you everything needed to pick up a task, implement it, and open a PR — without asking anyone.

> [!IMPORTANT]
> **If you are an AI agent, run this first before reading anything else:**
> ```sh
> context-pack --cwd . --format json
> ```
> That gives you the repo briefing. This file gives you the contributor layer on top of it.

---

## 1. Setup

```sh
# Build
cargo build

# Run tests (must all pass before opening a PR)
cargo test

# Lint (must be clean)
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Run the tool on itself — sanity check
cargo run -- --cwd . --format json
```

No other dependencies. This is a pure Rust project (`serde`, `serde_json` only).

---

## 2. Architecture Map

```
src/
├── main.rs            ← CLI entry, orchestrates the full pipeline
├── cli.rs             ← Argument parsing (AppConfig)
├── memory.rs          ← Learned repo memory metadata + staleness helpers
├── model.rs           ← Core types: AppConfig, ImportantFile, SignalCategory, etc.
├── select.rs          ← ★ THE HEART: file scoring, ranking, candidate selection (1831 LOC)
├── briefing.rs        ← Builds the high-level briefing (repo_summary, read_these_first, etc.)
├── detect.rs          ← Detects repo type (rust/node/python) and primary languages
├── walk.rs            ← Directory traversal and tree_summary generation
├── git.rs             ← Git status, branch info, changed files
├── ignore.rs          ← .gitignore-aware file filtering + --include/--exclude globs
├── render_markdown.rs ← Markdown output renderer
├── render_json.rs     ← JSON output renderer
├── mcp.rs             ← MCP server protocol (stdio JSON-RPC)
├── diff.rs            ← --diff mode: compare two context-pack JSON snapshots
├── docker_summary.rs  ← Docker/Compose file summarization
└── dependency_summary.rs ← Cargo/npm/pyproject dependency extraction
tests/
├── agent_briefing.rs  ← Integration tests: briefing correctness on fixture repos
└── markdown_snapshots.rs ← Snapshot tests: markdown output regression (UPDATE_EXPECT=1 to refresh)
```

### Key pipeline (follow `main.rs → build_context`):
1. `cli.rs` parses args → `AppConfig`
2. `ignore.rs` builds `IgnoreMatcher` (gitignore + CLI filters)
3. `walk.rs` generates `tree_summary`
4. `git.rs` collects git status + changed files
5. **`select.rs`** scans all files → scores → ranks → excerpts top N
6. `detect.rs` determines repo type
7. `briefing.rs` assembles the human-readable briefing
8. `render_markdown.rs` or `render_json.rs` serializes output

### Scoring system (in `select.rs → classify()`):
| Category | Base Score |
|---|---|
| `Instructions` (AGENTS.md, CLAUDE.md…) | 1000 |
| `Overview` (README, llms.txt) | 900 |
| `Manifest` (Cargo.toml, package.json) | 820 |
| `Build` (Makefile, Dockerfile) | 760 |
| `ChangedSource` | 740 |
| `EntryPoint` (main.rs, index.ts) | 700 |
| `Config` (.env.example) | 660 |
| `SupportingDoc` | 520 |

Bonuses added on top: `+40` repo root, `+15` shallow path, `+20` compact file (≤8 KB), `+90` changed source, language-aware bonus.

---

## 3. How to Pick a Task

1. Check [open issues](https://github.com/Rothschildiuk/context-pack/issues) labeled `agent-task`
2. Pick from the curated list below
3. Or read `docs/LAYERED_CONTEXT_ISSUES.md` for a concrete layered-context backlog
4. Or read `CONTEXT_PACK_PLAN.md` for the full roadmap and pick a near-term item
5. For context-artifact workflow details, read `docs/PROJECT_CONTEXT_WORKFLOW.md`

---

## 4. Good Agent Tasks

These are self-contained, well-scoped, and testable. Each one says which files to touch.

### 🟢 Small (S) — good first contributions

**S1: Add `CONTRIBUTING.md` to `supporting_doc_reason` in `select.rs`**
The file you are reading now should be auto-selected by the tool. `CONTRIBUTING.md` is not currently boosted.
- File: `src/select.rs` → `supporting_doc_reason()`
- Add a match arm for `"CONTRIBUTING.md"` → reason: `"contribution guide"`
- Add a fixture test in `tests/agent_briefing.rs`

**S2: Add token count to `--format json` output**
The markdown output already has an approximate token estimate. The JSON output does not surface it in the `notes` field in a standard way.
- File: `src/render_json.rs`
- Add `"approx_tokens": <n>` to the top-level JSON object
- Add a test that the field is present and numeric

**S3: Penalize large binary files from excerpt selection**
Files >100 KB that are binary should be excluded before excerpt budget is allocated.
- File: `src/select.rs` → `process_file()`
- Check `byte_len` before calling `score_candidate` for binary-looking extensions
- Add a test with a fixture that has a large binary file

---

### 🟡 Medium (M) — stronger impact

**M1: Add `--diff` support for comparing briefing quality**
Currently `--diff` (`src/diff.rs`) compares two JSON files. Improve it to also show which files appeared/disappeared between snapshots.
- Files: `src/diff.rs`, `src/cli.rs`
- Output: `added_files: [...]`, `removed_files: [...]`, `unchanged_files: [...]`
- Update snapshot test

**M2: `context-pack init` generates a starter `AGENTS.md`**
When a repo has no `AGENTS.md`, `context-pack --init-agents` should generate a minimal one, pre-filled with the repo name, detected language, and top 3 entrypoints from the briefing.
- Files: `src/main.rs`, `src/cli.rs`
- Template should be readable by agents and encourage contribution

**M3: Quality score in JSON output**
Add a `"briefing_quality_score": <0-100>` field based on: `AGENTS.md` present (+25), `README.md` present (+20), tests found (+20), entrypoints detected (+20), git available (+15).
- Files: `src/briefing.rs`, `src/render_json.rs`, `src/model.rs`
- Add a test asserting the score is in range and reflects presence/absence of signals

---

### 🔴 Large (L) — high impact

**L1: `--profile review` mode**
A "review" profile that focuses on `--changed-only` files + adjacent test files + the scoring context around them. Ideal for PR review agents.
- Files: `src/cli.rs`, `src/select.rs`, `src/model.rs`
- Needs end-to-end integration test with a fixture repo that has changed files + tests

**L2: Monorepo entrypoint detection**
For repos with `packages/`, `apps/`, `services/` directories, detect sub-package manifests and surface them as separate entrypoints in `likely_entry_points`.
- File: `src/briefing.rs`, `src/detect.rs`
- Add fixture test with a mock monorepo layout

---

## 5. PR Checklist

Before opening a PR, confirm:

- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo fmt --check` — no formatting changes needed
- [ ] If you changed markdown output: run `UPDATE_EXPECT=1 cargo test` to refresh snapshots, commit the updated snapshots
- [ ] PR title format: `feat: <short description>` or `fix: <short description>`
- [ ] PR description includes: what changed, which files modified, how you tested it

---

## 6. Test Guide

```sh
# Run all tests
cargo test

# Run a specific test
cargo test agent_briefing

# Refresh markdown snapshots after output changes
UPDATE_EXPECT=1 cargo test

# Run with output to debug
cargo test -- --nocapture
```

Fixtures live in `tests/` alongside `agent_briefing.rs` and `markdown_snapshots.rs`. When adding a new heuristic, add a corresponding fixture test.

---

## 7. Where to Add New Heuristics

All file classification lives in `src/select.rs`:

- **New file type to boost** → `classify()` → add branch before the fallthrough
- **New "supporting doc" type** → `supporting_doc_reason()` → add match arm
- **New entrypoint pattern** → `is_entrypoint_file()` → add filename check
- **New build file** → `is_build_file()` → add match arm
- **New manifest file** → `is_manifest()` → add match arm

All score constants are also in `select.rs`. Keep new base scores consistent with the table in Section 2.

---

## Questions?

Open an issue with label `question`. For faster response, include the output of:
```sh
context-pack --cwd . --format json
```
