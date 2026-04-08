---
paths:
  - "**/*.test.*"
  - "**/*_test.*"
  - "test/**"
  - "tests/**"
  - "src/**/tests.rs"
---

# Testing conventions

- Use `tokio::test` for async tests.
- Prefer unit tests inside modules for internal logic.
- Integration tests go in `tests/` directory.
- Ensure `make test` passes before considering a task complete.
