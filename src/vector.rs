use lancedb::connect;
use anyhow::Result;
use std::fs;

pub async fn init_vector_db() -> Result<lancedb::Connection> {
    let uri = "data/lancedb";
    fs::create_dir_all("data")?;
    let conn = connect(uri).execute().await?;
    Ok(conn)
}
