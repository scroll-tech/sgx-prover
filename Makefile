.PHONY: build_std lint

build_std:
	STD_MODE=true cargo sgx build

lint:
	cargo check --workspace
	cargo clippy --no-deps -- -D warnings
	cargo fmt --all