# AI Agent Guide

This guide is for coding agents working in specialized repositories such as cryptography, formal methods, consensus-critical runtimes, and mixed-language research codebases.

## Primary goals

- preserve correctness before velocity
- avoid speculative edits in consensus-critical or proof-related paths
- prioritize high-signal architecture and build docs before touching source files

## First-pass workflow

1. Run `context-pack --cwd <repo>` first.
2. Read root guidance in this order when present:
   - `AGENTS.md`
   - `README.md`
   - domain docs such as `ARCHITECTURE.md`, `RUNBOOK.md`, `OPERATIONS.md`
3. Identify the real execution surface:
   - runtime/library implementation (often C/Rust/Go)
   - proof/specification layer (often Coq)
   - generators/tooling (often Haskell/Python/Node)
4. Confirm build and test entrypoints before proposing code changes.

## Rules for specialized repositories

- Treat C and proof files as high-risk: require minimal diffs and clear invariants.
- Prefer additive changes and avoid broad refactors in security-sensitive logic.
- Keep language boundaries explicit (runtime vs proofs vs generators).
- If manifests for one ecosystem exist only in submodules or vendor trees, do not assume they define the repository's main stack.
- In mixed-language repositories, report which layer your change impacts.

## Output quality bar for agents

- State assumptions explicitly.
- Cite exact files used for decisions.
- Explain why selected files are relevant (use `reason`/`why` signals from context-pack output).
- Include validation steps and what was actually executed.

## Recommended context-pack usage

- Orientation: `context-pack --cwd <repo>`
- Review mode: `context-pack --cwd <repo> --profile review`
- Broad map for large trees: `context-pack --cwd <repo> --max-files 20 --max-depth 6`
- Stable baseline: `context-pack --cwd <repo> --no-language-aware`

## Anti-patterns

- Starting from random deep source files without reading guidance docs.
- Treating all files equally in consensus-critical repositories.
- Over-weighting incidental `package.json`/`tsconfig.json` files in nested third-party or tooling directories.
- Reporting confident conclusions without stating evidence paths.
