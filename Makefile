.PHONY: fmt clippy build test

fmt:
	cargo +nightly fmt

clippy:
	cargo clippy --workspace --all-targets

build:
	cargo build

test:
	cargo test -- --nocapture