NIX_RUN := $(if $(filter $(IN_NIX_SHELL),),nix develop --command ,)

.PHONY: help build run test fmt fmt-check lint check clean run-examples init

# Default task: Show help
help:
	@echo "Available tasks:"
	@echo "  build    - Compile the Rust project"
	@echo "  run      - Run the project via cargo (use ARGS=...)"
	@echo "  test     - Run all tests"
	@echo "  fmt      - Format all code (Rust, TOML, MD, JSON, YAML)"
	@echo "  lint     - Run all lints (Rust, Python, Markdown)"
	@echo "  check    - Run format check, lint, and tests (CI baseline)"
	@echo "  clean    - Remove build artifacts"
	@echo "  run-examples EXAMPLE=name - Run a specific async example"

build:
	$(NIX_RUN) cargo build

run:
	$(NIX_RUN) cargo run -- $(ARGS)

test:
	$(NIX_RUN) cargo test

fmt:
	$(NIX_RUN) cargo fmt
	$(NIX_RUN) taplo fmt
	$(NIX_RUN) prettier --write "**/*.{md,json,yaml,yml}"
	$(NIX_RUN) uv run ruff format .

fmt-check:
	$(NIX_RUN) cargo fmt -- --check
	$(NIX_RUN) taplo fmt --check
	$(NIX_RUN) prettier --check "**/*.{md,json,yaml,yml}"

lint:
	$(NIX_RUN) cargo clippy -- -D warnings
	$(NIX_RUN) uv run ruff check .
	$(NIX_RUN) find . -name "*.md" -not -path "./target/*" -not -path "./.venv/*" -exec uv run pymarkdown -c .pymarkdown.json scan {} +

check: fmt-check lint test

run-examples:
	$(NIX_RUN) cargo run --example $(EXAMPLE)

clean:
	$(NIX_RUN) cargo clean
	rm -rf target/

init:
	mise install
	uv venv --python 3.14
	uv add ruff mdformat-gfm pymarkdownlnt --dev
	mkdir -p scripts kb/topics
