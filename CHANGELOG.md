# Changelog

All notable changes to `context-pack` will be documented in this file.

The format is intentionally lightweight and release-focused.

## [0.7.0] - 2026-09-27

A rewrite of the briefing around what an agent needs in its first minute in a repository.

### Added

- **Commands** section: build/test/lint/format/run commands from Makefile, justfile, Taskfile, package scripts, ecosystem defaults, and the commands CI runs on pull requests
- **Entry points** declared by manifests (`[[bin]]`, npm `bin`/`main`/`exports`, `[project.scripts]`, `cmd/*/main.go`, Spring `*Application`, `Program.cs`, Dockerfile `CMD`)
- **Key files** ranked by size, git change frequency, and use by an entry point
- **Workspace** section grouping monorepo packages, excluding test fixtures
- **Layout** directory map with file counts, languages, and roles, replacing the raw tree
- Instruction files for Claude Code, Cursor, Copilot, Gemini CLI, Windsurf, Cline, aider, and agent skills, plus nested scoped `AGENTS.md`
- Branch changes and commit count versus the base branch; recent commits
- Project description from manifests or the README introduction
- `memory add "<note>"` command and `add_memory_note` MCP tool
- `context check` now fails when artifacts were generated from a different `HEAD`
- `UPDATE_EXPECT=1` support for snapshot tests

### Changed

- One gitignore-aware walk (the `ignore` crate) replaces four separate traversals and the hand-written gitignore parser; nested `.gitignore` files are respected and the whole repository is scored instead of the first 2,400 files in alphabetical order
- Path roles keep `examples/`, `docs_src/`, fixtures, benchmarks, vendored and generated code out of entry points and key files; Maven/Gradle package paths under `src/main/java` are always source
- `memory refresh` only updates the metadata block and never rewrites notes; memory notes are shown in every briefing
- Excerpts are budget filler: the root instruction file and declaration outlines with line numbers
- Secret redaction matches key names by word (`DB_PASSWORD`, `apiKey`) instead of substrings, so `keywords` or `tokenizer` are no longer redacted
- MCP tools return plain markdown text (and `structuredContent` only for JSON) instead of markdown wrapped in a JSON string
- `--cwd` pointing at a subdirectory of a git repository reports paths relative to it
- JSON output is serialized from one model; `schema_version` is now `2.0`
- Default budget is 6000 bytes and 8 key files; profiles are `compact`, `deep`, and `review`
- `--help` works after any subcommand

### Removed

- `--format viking`, `--diff-from`/`--diff-to`, `--minify`, `--max-depth`, `--no-language-aware`, `--no-tests`, and the `onboarding`/`incident` profiles
- Scores, token estimates, timing, and budget arithmetic from the markdown output
- promptfoo evals (covered by the Rust test suite), and outdated planning and marketing documents

### Fixed

- `get_file_excerpt` could read files outside the repository (`../`, absolute paths)
- Files referenced from entry points were labelled "explicitly included" without `--include`
- Changes to files the old classifier did not recognise (YAML, SQL, CI config, ...) were dropped from active work

## [0.6.0] - 2026-03-19

### Added

- command-oriented CLI entrypoints such as `context-pack review`, `context-pack memory refresh`, and `context-pack context refresh`
- built-in `context refresh` and `context check` workflows for generating and validating `.context-pack/PROJECT_CONTEXT.{md,json}` plus `.context-pack/memory.md`
- repo-memory freshness metadata and stale warnings when `.context-pack/memory.md` is older than 7 days and repository activity continued
- layered-context and project-context workflow documentation for agent-oriented onboarding and handoff

### Changed

- help output is now organized around common workflows first, with advanced flags separated below
- CI now smoke-tests project context artifact generation and validation directly through the CLI

## [0.5.1] - 2026-03-15

### Fixed

- `--minify` now applies only to source excerpts, preserving indentation-sensitive build snippets (for example, `Makefile` recipes)
- Rust local dependency extraction now resolves `use super::...` imports to the correct module directory and trims trailing semicolons in `use crate::...` / `use super::...` paths
- integration coverage now verifies dependency boosting with `--changed-only`, `super::` resolution, and Makefile behavior under `--minify`

## [0.5.0] - 2026-03-15

### Added

- smart minification (`--minify`) to optimize tokens for AST-heavy or comment-heavy files
- local dependency tracking to boost scores and include local imports connected to changed source and entrypoints

### Changed

- extracted dependency paths are natively treated as explicitly included signals and scored accordingly

## [0.4.4] - 2026-03-15

### Added

- `--quiet` flag for briefing-only output without excerpts, tree, or git detail sections
- new `--profile` presets: `compact` and `deep`
- regression tests for `compact`, `deep`, and `quiet` behaviors
- elapsed render timing note (`elapsed_ms`) in output notes

### Changed

- MCP `brief_repo` schema and validation now support `compact`/`deep` profiles and the `quiet` boolean
- markdown snapshot normalization now ignores volatile elapsed timing so tests stay stable

## [0.4.3] - 2026-03-14

### Changed

- improved language heuristics for specialized C/Coq/Haskell repositories
- fixed repository-shape prioritization so C+Coq projects are no longer misclassified as Node-first
- expanded AI-agent documentation with a dedicated guide for specialized repositories
- aligned README status and added troubleshooting for version/flag mismatches

## [0.4.2] - 2026-03-14

### Added

- language-aware scoring with explicit `why` reasoning in markdown and JSON outputs
- optional `--no-language-aware` switch to disable language boosts
- profile presets via `--profile onboarding|review|incident`
- `schema_version` field in JSON output for stable machine parsing
- artifact comparison mode via `--diff-from <path> --diff-to <path>`

## [0.4.1] - 2026-03-13

### Fixed

- markdown snapshot tests now normalize the approximate token note so release verification stays stable across platforms

## [0.4.0] - 2026-03-13

### Added

- Codex plugin scaffold with a bundled `context-pack` skill
- local MCP server mode with `brief_repo`, `init_memory`, and `refresh_memory` tools
- plugin publication assets and a `make plugin-check` smoke test for metadata and MCP validation

### Changed

- release and installation docs now describe the plugin and MCP workflow alongside the CLI

## [0.3.2] - 2026-03-13

### Changed

- improved `Hotspots` ranking inside bootstrapped repo memory drafts
- memory bootstrap now prioritizes entry points, changed source, large code files, and production source files above manifests and build files in the `Hotspots` section

## [0.3.1] - 2026-03-13

### Changed

- `--init-memory` now generates a prefilled repo memory draft instead of an almost empty template
- bootstrapped memory files now include purpose, read-first files, entry points, hotspots, caveats, and operational notes derived from the current repo context

## [0.3.0] - 2026-03-13

### Added

- support for `llms.txt` as an AI-facing repo summary signal
- support for `.clio/instructions.md` as tool-specific agent instructions
- stronger recognition of operational and agent-workflow docs such as `MEMORY.md`, `SANDBOX.md`, `REMOTE_EXECUTION.md`, `PERFORMANCE.md`, and `MULTI_AGENT_COORDINATION.md`

### Changed

- briefing heuristics now better support CLIO-style repositories with guidance spread across root docs, hidden tool directories, and AI-facing summaries
- README now documents the expanded guidance surface beyond `AGENTS.md`

## [0.2.5] - 2026-03-13

### Added

- `--init-memory` to create a `.context-pack/memory.md` template in one command
- `make init-memory` shortcut for bootstrapping learned repo memory from the project root

### Changed

- README now documents the learned repo memory bootstrap flow
- release examples now point at the latest `0.2.5` version

## [0.2.4] - 2026-03-13

### Added

- support for learned repo memory files as high-signal briefing inputs
- automatic detection of `REPO_MEMORY.md` at the repository root
- automatic detection of `.context-pack/memory.md` for tool-specific learned notes

### Changed

- repo memory files are now surfaced alongside `AGENTS.md`, manifests, entry points, and current git context
- README and roadmap messaging now describe token savings, fresh-thread workflows, and learned repo memory patterns

### Notes

- this release is especially aimed at older or messier repositories where useful operational knowledge does not fully exist in repo-authored docs yet
