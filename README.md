# context-pack

`context-pack` is a first-pass repository briefing generator for coding agents.

Point it at a repo, get a compact brief with the files, entrypoints, guidance docs, and active changes that matter first. It is built for the first minutes in an unfamiliar codebase, when agents usually waste time reading the wrong files or hauling too much low-signal context into the prompt.

Use it when `tree`, `rg`, and `git diff` are technically available but still leave too much orientation work to the model or the human driving it.

## Status

`context-pack` is currently an alpha CLI. The current release line is `0.5.x`.

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

The generated notes also include an approximate token count so you can quickly judge prompt weight before handing the bundle to a model.

## Why Context Density Matters

The point is not to maximize context size. The point is to maximize context density.

Too little context and the agent misses the entrypoint, ignores repo rules, or edits the wrong file. Too much context and the signal gets diluted by logs, boilerplate, and low-signal files.

`context-pack` is designed around that tradeoff:

- keep stable repo guidance visible
- surface active work separately from long-lived project rules
- point at the next files worth retrieving instead of dumping the whole repo
- keep fresh-thread handoff cheap enough to reuse

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
- language-aware ranking that boosts source and entrypoint signals for the top detected repository languages
- transparent `why` reasons per selected file in both markdown and JSON output

For specialized repositories (formal methods, cryptography, consensus-critical runtimes), see [`docs/AI_AGENT_GUIDE.md`](./docs/AI_AGENT_GUIDE.md) for a stricter agent workflow.

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
curl -LO https://github.com/Rothschildiuk/context-pack/releases/download/v<latest>/context-pack-v<latest>-<target>.tar.gz
tar -xzf context-pack-v<latest>-<target>.tar.gz
./context-pack --version
```

Install with Homebrew directly from this repository:

```bash
brew tap Rothschildiuk/context-pack https://github.com/Rothschildiuk/context-pack.git
brew install Rothschildiuk/context-pack/context-pack
```

Install directly from GitHub with Cargo:

```bash
cargo install --git https://github.com/Rothschildiuk/context-pack.git
```

Or run it from a local clone:

```bash
git clone https://github.com/Rothschildiuk/context-pack.git
cd context-pack
cargo run -- --help
```

## Quick Start

The simplest way to use `context-pack` is to think in workflow commands first and flags second.

Quick commands:

- `context-pack brief`
- `context-pack changed`
- `context-pack review`
- `context-pack incident`
- `context-pack memory-init`
- `context-pack memory-refresh`
- `context-pack json`

Generate a full repository brief:

```bash
context-pack brief --cwd .
```

Focus only on active work:

```bash
context-pack changed --cwd .
```

Use a preset profile:

```bash
context-pack review --cwd .
```

Available profiles:
- `onboarding`: default first-pass behavior
- `review`: changed-only focus with compact output (`--no-tree`) and a larger file shortlist
- `incident`: review-style focus plus a larger output budget for urgent triage

Disable language-aware ranking boosts (deterministic baseline behavior):

```bash
context-pack --cwd . --no-language-aware
```

Create a learned repo memory template:

```bash
context-pack --cwd . --init-memory
```

Regenerate the learned repo memory draft from the current repository state:

```bash
context-pack --cwd . --refresh-memory
```

Generated `.context-pack/memory.md` files now record `created_at_*` and `refreshed_at_*` metadata. When repo memory is older than 7 days and git activity continued, `context-pack` warns that the memory may be stale.

Generate machine-friendly JSON:

```bash
context-pack --cwd . --format json
```

Generate hierarchical Viking JSON (`L0`/`L1`/`L2`) for OpenViking-style integrations:

```bash
context-pack --cwd . --format viking
```

Schema details and tier descriptions live in `docs/schema/Viking.md`.

Future layered-output direction is documented in `docs/schema/LayeredContext.md`.

Generate a tighter bundle and check the approximate token weight in `Notes`:

```bash
context-pack --cwd . --changed-only --no-tree
```

Check the installed program version:

```bash
context-pack --version
```

## Troubleshooting

`unknown flag '--profile'` or `unknown flag '--diff-from'`:

- you are likely running an older installed binary
- verify with `context-pack --version`
- update from releases/Homebrew, or run the local source build with:

```bash
cargo run -- --cwd .
```

Output does not match expected repository shape:

- try `--no-language-aware` for a deterministic baseline
- try `--max-files 20 --max-depth 6` for broader context in large/specialized repositories

## Codex Plugin

This repository now includes a root-level Codex plugin scaffold:

- [`.codex-plugin/plugin.json`](./.codex-plugin/plugin.json)
- [`.mcp.json`](./.mcp.json)
- [`skills/context-pack/SKILL.md`](./skills/context-pack/SKILL.md)

The current plugin packages the `context-pack` skill and a local MCP server surface so Codex can learn and call a consistent repository-briefing workflow:

- run `context-pack` before a manual tree walk
- choose `--changed-only`, `--format json`, or tighter budgets based on the task
- use `.context-pack/memory.md` when repo knowledge needs to persist
- call `get_context`, `get_changed_context`, `get_file_excerpt`, `init_memory`, and `refresh_memory` over MCP when the plugin is installed

The MCP server runs from the same binary:

```bash
context-pack --mcp-server
```

This gives the plugin a real tool surface today, while still leaving room for future hook packaging once there is a concrete trigger design to ship.

To validate the plugin locally:

```bash
make plugin-check
```

That check verifies the plugin metadata, confirms referenced files exist, and smoke-tests the MCP server with `initialize`, `tools/list`, and `get_context`.

Marketplace note: the public plugin format is visible, but a public third-party submission flow was not documented in the official sources I checked. This repository is therefore prepared as a clean plugin source with local validation and publication-ready metadata, even if final catalog submission rules are still evolving.

## What You Get

- a compact first-pass brief instead of a raw file dump
- a context-dense working set instead of maximum raw context
- prioritized files and excerpts instead of an unranked tree walk
- repo instructions, manifests, and entrypoints surfaced together
- learned repo memory surfaced alongside repository-authored docs
- current git context included when it matters
- git changes labeled with compact status codes and short diff hints
- markdown for copy/paste workflows and JSON for automation
- explicit `why` arrays in JSON to explain each selected file score
- `schema_version` in JSON output for stable downstream parsing
- MCP tools return versioned `structuredContent` (`schemaVersion`) for stable agent parsing
- an approximate token estimate for the generated bundle

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

To generate the full local context-artifact set used by agents in this repository:

```bash
context-pack context refresh --cwd .
context-pack context check --cwd .
```

The generated file includes explicit creation and refresh timestamps so humans and agents can tell when the memory was last consolidated.

This is especially useful on older repositories where test coverage, logging, or repo docs are too weak to carry the full context on their own.

Longer term, `.context-pack/memory.md` is best thought of as one layer in a broader memory architecture:

- stable project rules and repo-level facts
- active working-set hints for the current task
- retrieval pointers for the next files worth opening
- short handoff notes that survive fresh threads without replaying the whole chat

The current repository workflow for this direction is documented in `docs/PROJECT_CONTEXT_WORKFLOW.md`.

## What It Captures

- repo type and primary languages
- current git changes and branch context
- compact git change labels such as `A`, `M`, and `D`, plus short diff hints when available
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

Compare two previously saved brief artifacts:

```bash
context-pack --diff-from brief-old.md --diff-to brief-new.md
```

Estimate whether a compact brief will fit comfortably into a fresh agent thread:

```bash
context-pack --cwd /path/to/repo --changed-only --no-tree
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
- optimized for context density rather than maximum token usage

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
git push origin v0.4.3
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
- generated bundles report an approximate token count in `Notes`.
- Rust is required to build from source, but not required for end users who install from GitHub Releases or Homebrew.
