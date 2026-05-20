## Objective
Harden Rust understanding through systematic lessons and practical refactoring in Rosemary.

## Status
DONE

## Completed Steps
- [x] Analyze project architecture and initialize KB.
- [x] Ingest existing design documents.
- [x] Refactor `VectorStore` to use slice references (`&[Chunk]`).
- [x] Fix test concurrency issues using in-memory databases and schema modularization.
- [x] Create and ingest lessons on Memory Layout and Database Isolation.
- [x] Implement the `Snippet<'a>` refactor in `src/recall.rs` to practice explicit lifetimes and eliminate unnecessary String clones.
- [x] Update `main.rs` and tests to use the new zero-copy `RecallData` pattern.
- [x] Build an interactive TUI dashboard using Ratatui and Crossterm (`rosemary browse`).

## Next Action
Session complete. User to test `rosemary browse`.

## Last Updated
2026-05-20
