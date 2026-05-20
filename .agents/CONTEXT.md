# Project Context: Rosemary

## Overview
A personal knowledge base CLI designed for agent-assisted learning and recall, and an async Rust learning project.

## Hard Rules
- **Markdown First:** All topic content must be stored in `kb/topics/` with YAML frontmatter.
- **Local-First:** Use libSQL (`rosemary.db`) locally for PKM metadata and vectors.
- **Slugified Paths:** All file names and DB keys must use URL-safe slugs.
- **Task Management:** Always use `make <target>` for task execution.
- **Tool Isolation:** Do not install tools globally; use nix devShell or `make <target>`.
- **Session Continuity:** Use MCP `search_nodes` to load project context at session start.

## Tech Stack
- Rust (Edition 2024)
- libSQL (PKM Storage)
- sqlx/sqlite (Async learning)
- Ratatui (TUI Dashboard)
- Nix + Makefile (Environment & Tasks)
