NIX_RUN := $(if $(filter $(IN_NIX_SHELL),),nix develop --command ,)

.PHONY: fmt fmt-check lint test check clean run-examples

fmt:
	$(NIX_RUN) cargo fmt
	$(NIX_RUN) taplo fmt
	$(NIX_RUN) prettier --write "**/*.{md,json,yaml,yml}"

fmt-check:
	$(NIX_RUN) cargo fmt -- --check
	$(NIX_RUN) taplo fmt --check
	$(NIX_RUN) prettier --check "**/*.{md,json,yaml,yml}"

lint:
	$(NIX_RUN) cargo clippy -- -D warnings

test:
	$(NIX_RUN) cargo test

check: fmt-check lint test

run-examples:
	$(NIX_RUN) cargo run --example $(EXAMPLE)

clean:
	$(NIX_RUN) cargo clean
