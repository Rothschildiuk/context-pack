.PHONY: help guard-cargo run changed check build test fmt clippy clean

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make guard-cargo - Verify that the Rust toolchain is installed' \
		'  make run      - Run context-pack against the current repository' \
		'  make changed  - Run context-pack in changed-only mode' \
		'  make check    - Run cargo check' \
		'  make build    - Build the project in debug mode' \
		'  make test     - Run cargo test' \
		'  make fmt      - Run cargo fmt' \
		'  make clippy   - Run cargo clippy -- -D warnings' \
		'  make clean    - Remove build artifacts'

guard-cargo:
	@command -v cargo >/dev/null 2>&1 || { \
		printf '%s\n' \
			'error: cargo not found in PATH' \
			'install Rust with rustup: https://rustup.rs/' ; \
		exit 1; \
	}

run: guard-cargo
	cargo run -- --cwd .

changed: guard-cargo
	cargo run -- --cwd . --changed-only

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

clean: guard-cargo
	cargo clean
