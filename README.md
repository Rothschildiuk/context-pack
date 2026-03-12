# context-pack

`context-pack` is a compact Rust CLI that turns a repository into a high-signal context bundle for coding agents.

It is meant for the first minutes with an unfamiliar codebase: generate one brief, paste it into ChatGPT/Codex/Claude, and start from a better baseline.

## Status

`context-pack` is currently an alpha CLI. The current release line is `0.2.x`.

## Install

Download a prebuilt binary from GitHub Releases without installing Rust:

```bash
curl -LO https://github.com/<your-name>/context-pack/releases/download/v0.2.1/context-pack-v0.2.1-<target>.tar.gz
tar -xzf context-pack-v0.2.1-<target>.tar.gz
./context-pack --version
```

Install with Homebrew directly from this repository:

```bash
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

Generate machine-friendly JSON:

```bash
context-pack --cwd . --format json
```

Check the installed program version:

```bash
context-pack --version
```

## What It Captures

- repo type and primary languages
- current git changes and branch context
- high-signal files with excerpts
- likely entry points
- Docker and Compose signals
- dependency summaries from common manifests
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

Save JSON for editor or automation workflows:

```bash
context-pack --cwd /path/to/repo --format json --output repo-context.json
```

## Example Workflow With an AI

1. Run `context-pack --cwd /path/to/repo --changed-only`.
2. Paste the output into your AI tool.
3. Ask a concrete question such as:
   `Review the active work, explain the likely entry point, and tell me where to change X.`

## Development

```bash
make help
make check
make run
make changed
```

## Release

Push a semantic version tag to build release archives automatically:

```bash
git push origin v0.2.1
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

After the release is published, GitHub Actions also updates `Formula/context-pack.rb` on the default branch so Homebrew can install from this same repository without a separate tap repo.

## Notes

- `Cargo.toml` is enough for IntelliJ IDEA / RustRover to open this as a Cargo project.
- `.idea/` and `target/` are ignored by git.
- Program version comes from `Cargo.toml` and is available via `context-pack --version`.
- Rust is required to build from source, but not required for end users who install from GitHub Releases or Homebrew.
