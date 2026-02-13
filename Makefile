.PHONY: build install test clean release dev

BINARY := octo-code
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

build:
	cargo build --release

install:
	cargo install --path crates/octo-cli

test:
	cargo test --workspace

check:
	cargo check --workspace
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean

dev:
	cargo watch -x 'build --release'

# Cross-compile for specific targets
release-linux-x86:
	cross build --release --target x86_64-unknown-linux-musl

release-linux-arm:
	cross build --release --target aarch64-unknown-linux-musl

release-macos-x86:
	cargo build --release --target x86_64-apple-darwin

release-macos-arm:
	cargo build --release --target aarch64-apple-darwin

# Package release binaries
dist: build
	@mkdir -p dist
	@cp target/release/$(BINARY) dist/$(BINARY)
	@echo "Binary at dist/$(BINARY)"
