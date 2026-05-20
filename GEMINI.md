<!-- Generated from AGENTS.md — edit AGENTS.md, not this file -->

# Rosemary

- **Language**: Rust (Edition 2024)
- **Runtime**: Tokio (full)
- **Task Runner**: `make` (Nix-wrapped)
- **Conventions**:
  - `anyhow` for app errors, `thiserror` for library errors.
  - Idiomatic Rust (snake_case, PascalCase).
  - MPMC channels via `flume`.
- **Primary Commands**:
  - `make fmt`
  - `make lint`
  - `make test`
  - `make check`
  - `make run-examples EXAMPLE=<name>`
