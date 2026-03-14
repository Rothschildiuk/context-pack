# Changelog

All notable changes to `context-pack` will be documented in this file.

The format is intentionally lightweight and release-focused.

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
