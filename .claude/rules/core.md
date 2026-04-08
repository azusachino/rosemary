# Core Agent Rules

- Use `make <target>` for all daily operations (fmt, lint, test, check).
- Follow standard Rust idiomatic patterns (snake_case, PascalCase).
- Prefer `anyhow` for errors in examples/applications, `thiserror` for core library code.
- Never commit without explicit user permission.
- Update `AGENTS.md` or `.agents/CONTEXT.md` when core project structure or conventions change.
- Use Nix-native tools via the `Makefile` wrapper.
