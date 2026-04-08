# Agent Rules

- **DO**: Use `make <target>` for all task execution.
- **DO**: At session start, load MCP entities if available; skip `CURRENT_TASK.md` when MCP active.
- **DO**: At session end, write state to `[rosemary]:session` MCP entity; do not write `CURRENT_TASK.md` when MCP active.
- **DO**: Update this file when architecture or conventions change.
- **DO**: Dispatch sub-agents for independent parallel tasks by default.
- **DON'T**: Commit without user confirmation.
- **DON'T**: Use plan mode (write-plan → execute-plan) for small, well-scoped tasks.
- **DON'T**: Install tools globally; use nix devShell or `make <target>`.

# Project Context

- Rosemary is a learning-focused repo. Prioritize clarity and idiomatic Rust patterns.
- Follow "best and popular practices" as requested by the user.
- When adding new examples, update `AGENTS.md` if new crates or patterns are introduced.

# Tool Provisioning

- **Nix DevShell**: Primary tool source. Enter with `nix develop`.
- **Makefile**: Task runner wrapper ensuring `nix develop --command` is used when outside the shell.
- To add a new tool, add it to `flake.nix` in the `devShells.default.packages` list.
