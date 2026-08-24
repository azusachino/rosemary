# Changelog

## v0.6.4 — Physical storage reclamation

### Fixed

- A database that has existed since before the 0.6 rusqlite rewrite carries every superseded schema generation's tables in place — the original `mcp_*` schema, then the libSQL/Turso-era `chunks`/`topics` vector schema — because each rewrite only ever added its own tables and never dropped the ones it replaced. Combined with SQLite's default `auto_vacuum=NONE`, a long-lived database could be over 95% dead pages that no command ever targeted. Schema v5 drops those tables on upgrade and switches every database to `auto_vacuum=INCREMENTAL` (a one-time `VACUUM` for an upgrading database, the pragma alone for a fresh one); `purge --apply` now runs a bounded `PRAGMA incremental_vacuum` afterward so routine purges keep reclaiming space instead of only marking it free. See ADR 0003 for the full account, including why this was previously deferred.
- `PRAGMA auto_vacuum` only takes effect before a database's file header is first written, which `PRAGMA journal_mode=WAL` does as a side effect. `open_at` was setting `auto_vacuum` after switching to WAL, so it silently never took effect on a fresh database; the pragma now runs first.
- `compact --older-than` claimed to prune session Markdown files, but nothing has written to `.asobi/topics/sessions/` since sessions were excluded from the Markdown projection — the flag was dead code from before that exclusion. Removed; `compact` is sync-only now.

### Tooling

- Toolchain pins bumped: Rust 1.98.0, uv 0.12.5, bun 1.4.0, ruff 0.16.4; `criterion` to 0.8. Rust 1.98's clippy adds `chunks_exact_to_as_chunks`, which `new`/`link`'s pair/triple parsing now satisfies via `as_chunks`.

### Verification

Added `opening_a_pre_v5_database_drops_superseded_tables_and_enables_incremental_vacuum` and `applied_purge_reclaims_space_via_incremental_vacuum` to the backend contract suite; both caught the WAL-ordering bug above before it shipped. The physical shrink itself (60MB → 1.2MB, 96% of pages reclaimed) was verified against a real long-lived database copy, not just the synthetic fixture. `make check` passes: storage boundary, rustfmt, Prettier, Ruff, Clippy `-D warnings`, all Rust tests, CLI verifier, use cases, and benchmark compilation.

## v0.6.3 — Self-contained skills and reliable releases

### Added

- Skill install/sync now inlines a skill's local `.md`/`.markdown` references into its stored body, so a `SKILL.md` that is itself just a table of contents over sibling docs ships self-contained. Both markdown links (`[text](path)`) and backtick-quoted paths (`` `references/schemas.md` ``, the style Anthropic's own `skill-creator` uses) are followed. A reference to another skill's own entry point (`SKILL.md`/`index.md`) is never inlined — that stays a cross-skill reference to something installed as its own entity — and a link resolving outside the source checkout is never followed.
- `--subdir <path>` on `skills install`, and a matching `subdir = "..."` field on `[[skills.source]]` in `asobi.toml`, scope the install walk to one directory of a checkout. Some sources mirror every skill across several tool-specific directories (`.opencode/`, `.kiro/`, a canonical `skills/`, ...) with the same `name:` in each copy; `subdir` avoids the mirrors entirely instead of asking install to arbitrate between diverging copies.

### Fixed

- A source that declares the same skill `name:` in more than one file (a real pattern: mirrored copies under different tool-specific directories) used to fail with a confusing `Content missing for skill X`. It now fails with a specific error naming every colliding file, and — for `--select` — only when the actually-selected name collides, so an unrelated, unambiguous skill in the same source still installs.
- The release workflow's macOS binary upload had no retry, so a transient connect-timeout to `api.github.com` (observed directly in CI logs, alongside `mise-action` hitting the same timeout independently) failed the whole release. The GitHub release is now created once in `publish-crate`, and each platform's binary upload retries with backoff — `v0.6.2`'s release shipped without its macOS binary because of exactly this.

### Verification

Reference inlining and the duplicate-name fix were verified against real upstream skill repos (`mattpocock/skills`, `obra/superpowers`, `anthropics/skills`, `DietrichGebert/ponytail`, `addyosmani/agent-skills`, `jasonswett/llm-skills`), not just synthetic fixtures. `make check` passes: storage boundary, rustfmt, Prettier, Ruff, Clippy `-D warnings`, all Rust tests, CLI verifier, use cases, and benchmark compilation.

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
