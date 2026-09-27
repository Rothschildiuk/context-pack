.PHONY: help guard-cargo guard-python run changed init-memory refresh-memory refresh-context context-check plugin-check check build test fmt clippy snapshots clean

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make guard-cargo - Verify that the Rust toolchain is installed' \
		'  make run      - Run context-pack against the current repository' \
		'  make changed  - Run context-pack in changed-only mode' \
		'  make init-memory - Create a repo memory template in .context-pack/memory.md' \
		'  make refresh-memory - Mark .context-pack/memory.md notes as reviewed' \
		'  make refresh-context - Generate .context-pack/PROJECT_CONTEXT.{md,json} plus memory.md' \
		'  make context-check - Validate generated context artifacts' \
		'  make plugin-check - Validate plugin metadata and smoke-test the MCP server' \
		'  make check    - Run cargo check' \
		'  make build    - Build the project in debug mode' \
		'  make test     - Run cargo test' \
		'  make fmt      - Run cargo fmt' \
		'  make clippy   - Run cargo clippy --all-targets -- -D warnings' \
		'  make snapshots - Regenerate markdown golden files (review the diff)' \
		'  make clean    - Remove build artifacts'

guard-cargo:
	@command -v cargo >/dev/null 2>&1 || { \
		printf '%s\n' \
			'error: cargo not found in PATH' \
			'install Rust with rustup: https://rustup.rs/' ; \
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
	cargo run -- --cwd . memory init

refresh-memory: guard-cargo
	cargo run -- --cwd . memory refresh

refresh-context: guard-cargo
	cargo run -- --cwd . context refresh

context-check: guard-cargo
	cargo run -- --cwd . context check

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
	cargo clippy --all-targets -- -D warnings

snapshots: guard-cargo
	UPDATE_EXPECT=1 cargo test --test markdown_snapshots

clean: guard-cargo
	cargo clean
