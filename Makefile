# Makefile for Rosemary Project

.PHONY: help build run test fmt lint clean init

# Default task: Show help
help:
	@echo "Available tasks:"
	@echo "  build    - Compile the Rust project"
	@echo "  run      - Run the project via cargo"
	@echo "  test     - Run Rust tests"
	@echo "  fmt      - Format Rust, Python, and Markdown code"
	@echo "  lint     - Run Rust clippy, Python ruff, and Markdown lint"
	@echo "  clean    - Remove build artifacts"
	@echo "  init     - Initialize development environment (mise, uv, etc.)"

build:
	cargo build

run:
	cargo run -- $(ARGS)

test:
	cargo test

fmt:
	cargo fmt
	uv run ruff format .
	find . -name "*.md" -not -path "./target/*" -not -path "./.venv/*" -exec uv run mdformat {} +

lint:
	cargo clippy -- -D warnings
	uv run ruff check .
	find . -name "*.md" -not -path "./target/*" -not -path "./.venv/*" -exec uv run pymarkdown -c .pymarkdown.json scan {} +

clean:
	cargo clean
	rm -rf target/

init:
	mise install
	uv venv --python 3.14
	uv add ruff mdformat-gfm pymarkdownlnt --dev
	mkdir -p scripts kb/topics
