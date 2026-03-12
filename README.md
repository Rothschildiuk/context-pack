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
cargo run -- --cwd .
cargo run -- --cwd . --changed-only
```

## Notes

- `Cargo.toml` is enough for IntelliJ IDEA / RustRover to open this as a Cargo project
- `.idea/` and `target/` are ignored by git
- JSON output is still a stub in the current vertical slice
