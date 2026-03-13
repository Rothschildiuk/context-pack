# Changelog

All notable changes to `context-pack` will be documented in this file.

The format is intentionally lightweight and release-focused.

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
