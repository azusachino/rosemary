# Rosemary

## Project Overview

Rosemary is a Rust learning project focused on mastering async Rust patterns, networking, and best practices. It includes a variety of standalone examples, a toolkit for common async tasks, and exploration of standard library and ecosystem crates.

## Tech Stack & Architecture

- **Language**: Rust (Edition 2024)
- **Async Runtime**: `tokio` (full), `async-task`, `futures-lite`
- **Serialization**: `serde`, `serde_json`
- **Networking**: `reqwest`, `tokio-util`
- **Channels**: `flume` (MPMC)
- **Error Handling**: `anyhow`, `thiserror`
- **Tooling**: Nix + Makefile (Primary), `mise` (Fallback)

The project is structured as a hybrid library/examples repository:
- `src/`: Core library and learning modules.
- `examples/`: Standalone async Rust samples.
- `benches/`: Performance benchmarks.

## Build, Run & Test

All daily operations are managed via `make`:

- `make fmt`: Format all code (Rust, Markdown, TOML, YAML).
- `make lint`: Run `clippy` with pedantic checks.
- `make test`: Run all tests.
- `make check`: Run format check, lint, and tests (CI baseline).
- `make run-examples EXAMPLE=name`: Run a specific example from `examples/`.

If you have Nix installed, these commands automatically run within the `nix develop` environment.

## Coding Conventions

- **Naming**: Standard Rust naming (snake_case for functions/variables, PascalCase for types).
- **Error Handling**: Prefer `anyhow` for top-level application logic and `thiserror` for library-style domain errors.
- **Testing**: Table-driven tests where appropriate. Integration tests in `tests/` or embedded unit tests.
- **Formatting**: `rustfmt` for Rust, `taplo` for TOML, `prettier` for MD/JSON/YAML.

## Key Files & Entry Points

- `src/main.rs`: Main entry point (if any).
- `src/lib.rs`: Library definitions and common utilities.
- `Cargo.toml`: Project dependencies and configuration.
- `flake.nix`: Development environment definition.

## Quality Standards

- No warnings in `clippy`.
- All tests must pass.
- All code must be formatted with `make fmt`.
