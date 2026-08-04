# Changelog

## v0.6.2 — Declarative skill sync

### Added

- `asobi skills sync` reconciles installed skills with a `[skills]` block in the discovered `asobi.toml`, treating the config as the whole truth: declared sources are installed or refreshed, unselected skills are pruned, and skills from undeclared sources are removed.
- Selected skills are materialised to disk at `<path>/<source-slug>@<skill-name>/SKILL.md` alongside the graph copy, so agents can read them off the filesystem. Both halves of the directory name are slugified to lowercase kebab-case. `path` defaults to `.agents/skills`, relative to the declaring `asobi.toml`. On-disk pruning is scoped to the `@` naming convention, leaving vendored checkouts and hand-authored skills untouched.

### Changed

- `AsobiPaths` now carries the discovered workspace `root` and `config_file`, so project-relative content paths resolve consistently regardless of the working directory.
- `install_skills_from_dir` reports what it installed and pruned instead of returning unit.

### Tooling

- Tool versions (rust, uv, bun, ruff) are pinned in `.mise.toml`; `make init` provisions them and both workflows resolve the same file, so local and CI no longer drift. `flake.nix` and `flake.lock` are removed.
- `[tool.ruff]` is declared rather than inherited: without it ruff walks up out of the repo and adopts a parent directory's config, which reformatted the tree to width 100 when checked out inside such a workspace. Ruff 0.16 also widens the default rule set from 118 rules to 826.

### Verification

`make check` passes, including storage-boundary checks, Clippy, Rust tests, CLI integration checks, use cases, and benchmark compilation.

## v0.6.1 — Lean agent reads and safe retention

### Added

- Preview-first `purge` for stale terminal `session` and `task` entities, with transactional deletion, relation/FTS cleanup, JSON reports, and durable-knowledge safeguards.
- Shell completion generation for Bash, Elvish, Fish, PowerShell, and Zsh through `asobi completions <shell>`.

### Changed

- `graph` and `search` now return lean entity indexes: observations and skill bodies are lazy and available through explicit `show`, `export`, `backup`, or `skills show` operations.
- Updated the usage guide and retention ADR with the 0.6.1 maintenance and completion workflows.

### Verification

`make check` passes, including storage-boundary checks, Clippy, Rust tests, CLI integration checks, use cases, and benchmark compilation. Tarpaulin reports 62.60% line coverage (1,053/1,682).

## v0.6.0 — Curated SQLite graph storage

### Added

- Synchronous `api::v2` storage traits for graph, search, skills, snapshots, backups, maintenance, and task dispatch.
- Bundled SQLite through `rusqlite`, with WAL mode, foreign keys, bounded busy timeouts, and FTS5/BM25 keyword search.
- Durable task dispatcher: `tasks plan`, `list`, `dispatch`, `sync`, and `close` with nested help, lifecycle validation, and JSON response schemas.
- Atomic task dispatch: status transition, claimant truth, and dispatch observation commit together, so concurrent agents produce one winner.
- Graph-to-Markdown `compact` projection for durable knowledge topics.
- Contract, CLI, evil-input, edge-case, concurrent-process, daily-practice, and benchmark coverage.

### Removed

- libSQL/Turso and SQLx providers.
- Vector/document ingestion, semantic recall, and feature-gated product paths.
- The obsolete async v1 storage contract and provider-specific verification scripts.

### Verification

`make check` runs formatting, Clippy, all Rust tests, the CLI verifier, the daily use-case scenario, and storage-boundary checks. `cargo bench --no-run` verifies all benchmark targets; `make bench` executes them.

## v0.5.2 — Versioned CLI responses

- Added command-specific JSON Schema discovery through `schema` and `schema --command NAME`.
- Standardized structured errors and local-time tracing output.

## v0.5.1 — Leaner CLI build

- Reduced default CLI dependencies and tightened logging and formatting gates.

## v0.5.0 and earlier

- Established the standalone knowledge-graph CLI, SQLite-compatible graph schema, truths, observation history, lazy reads, skills, compact Markdown projections, portable JSON export/import, and local/XDG workspace layouts.
