use libsql::{Builder, Connection};
use anyhow::Result;

pub async fn init_db() -> Result<Connection> {
    let db = Builder::new_local("rosemary.db").build().await?;
    let conn = db.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS topics (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            slug TEXT NOT NULL,
            file_path TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    ).await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            entity_type TEXT
        )",
        (),
    ).await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS relations (
            from_id TEXT,
            to_id TEXT,
            relation_type TEXT,
            PRIMARY KEY (from_id, to_id, relation_type)
        )",
        (),
    ).await?;

    Ok(conn)
}
