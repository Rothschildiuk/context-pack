.PHONY: help guard-cargo guard-node guard-python run changed init-memory refresh-memory plugin-check check build test fmt clippy eval-promptfoo clean

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make guard-cargo - Verify that the Rust toolchain is installed' \
		'  make guard-node - Verify that Node.js tooling is installed' \
		'  make run      - Run context-pack against the current repository' \
		'  make changed  - Run context-pack in changed-only mode' \
		'  make init-memory - Create a repo memory template in .context-pack/memory.md' \
		'  make refresh-memory - Regenerate .context-pack/memory.md from current repo context' \
		'  make plugin-check - Validate plugin metadata and smoke-test the MCP server' \
		'  make check    - Run cargo check' \
		'  make build    - Build the project in debug mode' \
		'  make test     - Run cargo test' \
		'  make fmt      - Run cargo fmt' \
		'  make clippy   - Run cargo clippy -- -D warnings' \
		'  make eval-promptfoo - Run promptfoo regression evals' \
		'  make clean    - Remove build artifacts'

guard-cargo:
	@command -v cargo >/dev/null 2>&1 || { \
		printf '%s\n' \
			'error: cargo not found in PATH' \
			'install Rust with rustup: https://rustup.rs/' ; \
		exit 1; \
	}

guard-node:
	@command -v npx >/dev/null 2>&1 || { \
		printf '%s\n' \
			'error: npx not found in PATH' \
			'install Node.js to run promptfoo evals: https://nodejs.org/' ; \
		exit 1; \
	}

guard-python:
	@command -v python3 >/dev/null 2>&1 || { \
		printf '%s\n' \
			'error: python3 not found in PATH' \
			'install Python 3 to run plugin validation' ; \
		exit 1; \
	}

run: guard-cargo
	cargo run -- --cwd .

changed: guard-cargo
	cargo run -- --cwd . --changed-only

init-memory: guard-cargo
	cargo run -- --cwd . --init-memory

refresh-memory: guard-cargo
	cargo run -- --cwd . --refresh-memory

plugin-check: guard-cargo guard-python
	python3 scripts/validate_plugin.py

check: guard-cargo
	cargo check

build: guard-cargo
	cargo build

test: guard-cargo
	cargo test

fmt: guard-cargo
	cargo fmt

clippy: guard-cargo
	cargo clippy -- -D warnings

eval-promptfoo: guard-cargo guard-node
	PROMPTFOO_CONFIG_DIR=.promptfoo npx promptfoo@latest eval -c promptfooconfig.yaml

clean: guard-cargo
	cargo clean
