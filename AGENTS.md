# Rosemary

## Project Overview

Rosemary is a dual-purpose project:
1. **Personal Knowledge Base (PKM) CLI**: A high-performance, agent-assisted reactive memory for storing and recalling technical concepts using hybrid semantic/keyword search.
2. **Async Rust Masterclass**: A learning ground for advanced async patterns, networking, observability, and concurrency.

## Tech Stack & Architecture

- **Language:** Rust (Edition 2024), Python 3.14 (Scripts)
- **Database:** libSQL (PKM metadata/vectors) and sqlx/sqlite (Async learning)
- **Vectors:** LanceDB + FastEmbed (all-MiniLM-L6-v2)
- **Async Runtime:** `tokio` (full), `async-task`, `futures-lite`
- **Observability:** `tracing`, `metrics-exporter-prometheus`
- **Channels:** `flume` (MPMC)
- **CLI:** `clap` for command parsing
- **Tooling:** Nix + Makefile (Primary), `mise` (Fallback), `uv` for Python environments

## Build, Run & Test

All operations are managed via `make`:

- `make build`: Build the Rosemary binary.
- `make fmt`: Format all code (Rust, Markdown, TOML, YAML).
- `make lint`: Run `clippy` with pedantic checks.
- `make test`: Run all tests.
- `make check`: Run format check, lint, and tests (CI baseline).
- `make run-examples EXAMPLE=name`: Run a specific async example from `examples/`.

## Coding Conventions

- **Naming:** `snake_case` for functions/variables, `PascalCase` for types and traits.
- **Error Handling:** Use `anyhow::Result` for application-level errors and `thiserror` for library-level errors.
- **Async Patterns:** Prefer explicit composition and delegation over complex inheritance.
- **Staging Discipline:** Stage specific files; never use `git add -A`.

## Key Files & Entry Points

- `src/main.rs`: CLI entry point and command matching.
- `src/lib.rs`: Module declarations and common utilities.
- `src/db.rs`: libSQL schema and PKM database logic.
- `src/tui.rs`: Interactive dashboard implementation.
- `examples/`: Standalone async Rust samples.

## Quality Standards

- No warnings in `clippy`.
- All tests must pass.
- Markdown files must follow basic formatting rules.
