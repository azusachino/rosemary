---
title: Database Locking and Test Isolation in Rust
slug: rust-test-isolation
tags: [rust, testing, sqlite, concurrency]
---

# Handling Database Concurrency in Tests

## 1. The Problem: "Database is Locked"
When running `cargo test`, Rust executes tests in parallel by default. If multiple tests try to write to the same file-based SQLite database (`rosemary.db`), one will lock the file, causing others to fail with `SQLite failure: database is locked`.

## 2. The Solution: In-Memory Isolation
Use `:memory:` for the database URL during tests. This creates a fresh, isolated database in RAM for every test connection.

```rust
let conn = Builder::new_local(":memory:").build().await?.connect()?;
```

## 3. Hardening Tip: Schema Modularization
To make `:memory:` databases useful, extract your table creation logic into a reusable function that takes a `&Connection`.

```rust
// In src/db.rs
pub async fn init_db_on_conn(conn: &Connection) -> Result<()> {
    // CREATE TABLE IF NOT EXISTS ...
}
```

This allows both your production `init_db()` and your test suite to guarantee the same schema without sharing a file.
