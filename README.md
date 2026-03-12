# context-pack

`context-pack` is a compact Rust CLI that turns a repository into a high-signal context bundle for coding agents.

## Current Status

The project currently includes:

- CLI argument parsing
- repository tree walking with ignore rules
- basic `.gitignore` and `.ignore` support
- git status collection
- high-signal file selection and excerpt extraction
- Markdown bundle rendering

## Run

```bash
cargo run -- --help
cargo run -- --version
cargo run -- --cwd .
cargo run -- --cwd . --changed-only
cargo run -- --cwd . --format json
```

## Prerequisites

Install the Rust toolchain before running the project:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Make Targets

```bash
make guard-cargo
make help
make check
make run
make changed
```

## Notes

- `Cargo.toml` is enough for IntelliJ IDEA / RustRover to open this as a Cargo project
- `.idea/` and `target/` are ignored by git
- JSON output is available for automation and editor integrations
- Program version comes from `Cargo.toml` and is available via `context-pack --version`
