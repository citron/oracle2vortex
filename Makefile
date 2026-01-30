.PHONY: help build release clean test run install check

help:
	@echo "oracle2vortex - Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build    - Build debug version"
	@echo "  release  - Build optimized release version"
	@echo "  clean    - Clean build artifacts"
	@echo "  test     - Run tests"
	@echo "  check    - Check code without building"
	@echo "  install  - Install to ~/.cargo/bin"
	@echo "  run      - Run with example (requires Oracle)"

build:
	cargo build

release:
	cargo build --release

clean:
	cargo clean

test:
	cargo test

check:
	cargo check

install:
	cargo install --path .

run:
	@echo "Example usage (edit connection parameters):"
	@echo "cargo run -- -f examples/sample_query.sql -o output.vortex --host localhost --port 1521 -u hr -p password --sid XEPDB1"
