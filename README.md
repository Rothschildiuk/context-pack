# context-pack

`context-pack` is a first-pass repository briefing generator for coding agents.

Point it at a repo, get a compact brief with the files, entrypoints, guidance docs, and active changes that matter first. It is built for the first minutes in an unfamiliar codebase, when agents usually waste time reading the wrong files or hauling too much low-signal context into the prompt.

Use it when `tree`, `rg`, and `git diff` are technically available but still leave too much orientation work to the model or the human driving it.

## Status

`context-pack` is currently an alpha CLI. The current release line is `0.2.x`.

## Why This Exists

Coding agents often fail in the same predictable ways on a fresh repository:

- they start with a blind tree walk
- they miss `AGENTS.md`, `README.md`, or repo-specific instructions
- they burn prompt budget on low-signal files
- they edit near a symptom instead of at the actual entrypoint

`context-pack` turns that messy first pass into one small, deliberate briefing so the next question starts from the right files and the right constraints.

## Why Token Savings Matter

In many agent workflows, a fresh thread means paying the repo-orientation cost again.

That is especially visible in tools like Codex, ChatGPT, or Claude when a new session starts on the same project and the model re-reads repo structure, manifests, and random source files before it becomes useful.

`context-pack` helps reduce that repeated orientation spend by turning the first pass into a compact, reusable briefing instead of a full repo dump.

## Why Not Just `tree + rg + git diff`?

Those tools are necessary, but they are not a briefing.

- `tree` shows structure, not priority
- `rg` finds strings, not the best starting points
- `git diff` shows changes, not repo guidance or architectural entrypoints
- raw CLI output is usually too noisy to paste into a prompt unchanged

`context-pack` ranks and compresses the useful parts into a small bundle meant for immediate handoff to ChatGPT, Codex, Claude, or another agent.

## Why Not RAG or a Repo Indexer?

RAG and indexers are useful when you need broad semantic recall across a large codebase. `context-pack` solves a different problem:

- no indexing or embedding pipeline
- works directly from the local repository state
- captures current git context, guidance docs, and active changes
- keeps the first-pass output small enough for tight prompt budgets

Use RAG when you need deep retrieval. Use `context-pack` when you need a fast, deterministic repo brief before deeper work starts.

## Why Not Repo Instructions Alone?

Files like `AGENTS.md`, `CLAUDE.md`, and `README.md` are high-signal, but they are only part of the picture.

`context-pack` combines those docs with:

- tool-specific instructions such as `.clio/instructions.md`
- AI-facing summaries such as `llms.txt`
- learned repo memory files such as `REPO_MEMORY.md` or `.context-pack/memory.md`
- likely entrypoints
- current branch and changed-file context
- dependency and build signals
- selected excerpts from the files most worth reading next

That makes repo instructions more useful because they arrive together with the code context needed to act on them.

## Before / After

Without `context-pack`:

- the agent scans the tree
- opens a few large files at random
- misses `AGENTS.md`
- reads local IDE noise or low-signal config
- proposes a change in the wrong module

With `context-pack`:

- the agent sees `AGENTS.md`, `README.md`, manifests, and entrypoints first
- active work is summarized before the model starts exploring
- shared repo config is surfaced while local workspace noise stays out
- the next prompt can ask about the right module, test, or diff immediately

Typical result: less orientation drift, fewer wrong-file edits, and a much smaller first prompt.

## Who It Is For

- coding agents that need a fast repo briefing
- engineers who want a clean first-pass summary before asking an AI for help
- automation workflows that need compact markdown or JSON context
- fresh-thread workflows where repeated repo orientation burns too many tokens

## Who It Is Not For

- full-text semantic search across a large codebase
- long-lived indexing pipelines
- tools meant to replace `rg`, `git`, or your editor

## Install

Download a prebuilt binary from GitHub Releases without installing Rust:

```bash
curl -LO https://github.com/<your-name>/context-pack/releases/download/v0.2.5/context-pack-v0.2.5-<target>.tar.gz
tar -xzf context-pack-v0.2.5-<target>.tar.gz
./context-pack --version
```

Install with Homebrew directly from this repository:

```bash
brew tap Rothschildiuk/context-pack https://github.com/Rothschildiuk/context-pack.git
brew install Rothschildiuk/context-pack/context-pack
```

Install directly from GitHub with Cargo:

```bash
cargo install --git https://github.com/<your-name>/context-pack
```

Or run it from a local clone:

```bash
git clone https://github.com/<your-name>/context-pack.git
cd context-pack
cargo run -- --help
```

## Quick Start

Generate a full repository brief:

```bash
context-pack --cwd .
```

Focus only on active work:

```bash
context-pack --cwd . --changed-only
```

Create a learned repo memory template:

```bash
context-pack --cwd . --init-memory
```

Regenerate the learned repo memory draft from the current repository state:

```bash
context-pack --cwd . --refresh-memory
```

Generate machine-friendly JSON:

```bash
context-pack --cwd . --format json
```

Check the installed program version:

```bash
context-pack --version
```

## What You Get

- a compact first-pass brief instead of a raw file dump
- prioritized files and excerpts instead of an unranked tree walk
- repo instructions, manifests, and entrypoints surfaced together
- learned repo memory surfaced alongside repository-authored docs
- current git context included when it matters
- markdown for copy/paste workflows and JSON for automation

## Learned Repo Memory

`context-pack` can also surface learned repo knowledge that does not naturally live in the codebase itself yet.

Useful patterns:

- `AGENTS.md` for repo instructions
- `.clio/instructions.md` for tool-specific agent instructions
- `llms.txt` for AI-facing repo summaries
- `REPO_MEMORY.md` for accumulated operational knowledge
- `.context-pack/memory.md` for tool-specific learned notes

To bootstrap the tool-specific file:

```bash
context-pack --cwd /path/to/repo --init-memory
```

Or from the project root:

```bash
make init-memory
```

To overwrite the generated draft later:

```bash
make refresh-memory
```

This is especially useful on older repositories where test coverage, logging, or repo docs are too weak to carry the full context on their own.

## What It Captures

- repo type and primary languages
- current git changes and branch context
- high-signal files with excerpts
- likely entry points
- Docker and Compose signals
- dependency summaries from common manifests
- shared editor and IDE configs such as `.editorconfig`, VS Code tasks, and IntelliJ run configs
- a compact tree snapshot

## Common Use Cases

Repository onboarding:

```bash
context-pack --cwd /path/to/repo
```

Review the current branch before asking an AI for help:

```bash
context-pack --cwd /path/to/repo --changed-only
```

Start a fresh Codex or ChatGPT thread on an existing project without paying the full repo-orientation cost again:

```bash
context-pack --cwd /path/to/repo --no-tree
```

Save JSON for editor or automation workflows:

```bash
context-pack --cwd /path/to/repo --format json --output repo-context.json
```

## Example Workflow With an AI

1. Run `context-pack --cwd /path/to/repo --changed-only`.
2. Paste the output into your AI tool.
3. Ask a concrete question such as:
   `Review the active work, explain the likely entry point, and tell me where to change X.`

For fresh-thread workflows on the same repo, use the briefing as a compact orientation layer instead of asking the model to rediscover the codebase from scratch.

## Positioning Summary

`context-pack` is best thought of as the repo briefing layer for coding agents:

- lighter than RAG
- more directed than `tree`
- more reusable than ad hoc copy/paste from `rg` and `git diff`
- better aligned with prompt budgets than dumping raw repo state

## Development

```bash
make help
make check
make run
make changed
```

## Promptfoo Evals

`context-pack` now ships with a small `promptfoo` regression suite for briefing quality.
It runs the CLI against repository fixtures and asserts on the rendered output, so it is useful for catching ranking regressions, missing docs, and low-signal excerpts without calling a model API.

Run it with `npx`:

```bash
PROMPTFOO_CONFIG_DIR=.promptfoo npx promptfoo@latest eval -c promptfooconfig.yaml
```

Or use the Make target:

```bash
make eval-promptfoo
```

If you already built the binary and want to skip `cargo run` inside the eval harness:

```bash
PROMPTFOO_CONFIG_DIR=.promptfoo CONTEXT_PACK_BIN=./target/debug/context-pack npx promptfoo@latest eval -c promptfooconfig.yaml
```

## GitHub Workflow

This repository also uses GitHub-native tooling to keep feedback and releases structured:

- `CHANGELOG.md` for concise release tracking
- issue forms for bugs and feature requests
- GitHub release note categories via `.github/release.yml`
- `GITHUB_PLAYBOOK.md` for suggested Discussions, labels, and release habits

## Release

Push a semantic version tag to build release archives automatically:

```bash
git push origin v0.2.5
```

The release workflow builds:

- macOS Apple Silicon: `aarch64-apple-darwin`
- macOS Intel: `x86_64-apple-darwin`
- Linux Intel: `x86_64-unknown-linux-gnu`

Each tagged release publishes:

- compressed binary archives
- per-asset `sha256` files
- a combined `SHA256SUMS`
- a generated `context-pack.rb` Homebrew formula
- release notes tracked in `CHANGELOG.md`

After the release is published, GitHub Actions also updates `Formula/context-pack.rb` on the default branch so Homebrew can install from this same repository without a separate tap repo.

## Notes

- `Cargo.toml` is enough for IntelliJ IDEA / RustRover to open this as a Cargo project.
- `.idea/` and `target/` are ignored by git.
- Program version comes from `Cargo.toml` and is available via `context-pack --version`.
- Rust is required to build from source, but not required for end users who install from GitHub Releases or Homebrew.
