use crate::{db::search_fts, embed::EmbeddingProvider, vector::{VectorStore, SearchResult}};
use anyhow::Result;
use libsql::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Snippet<'a> {
    pub text: &'a str,
    pub score: f32,
}

#[derive(Debug)]
pub struct RecallResult<'a> {
    pub topic_id: &'a str,
    pub title: &'a str,
    pub file_path: &'a str,
    pub snippet: Snippet<'a>,
    pub score: f32,
}

/// Holds owned search results to allow lifetime-bound views (RecallResult)
pub struct RecallData {
    pub ann_results: Vec<SearchResult>,
    pub fts_results: Vec<(String, String, String, f64)>,
    pub metadata: HashMap<String, (String, String)>,
}

impl RecallData {
    pub fn ranked<'a>(&'a self, top_k: usize) -> Vec<RecallResult<'a>> {
        // topic_id -> (merged_score, best_snippet, title, path)
        let mut scores: HashMap<&str, (f32, Snippet<'a>, &str, &str)> = HashMap::new();

        // 1. Process ANN results (weight 0.7)
        for r in &self.ann_results {
            let entry = scores.entry(&r.topic_id).or_insert((
                0.0,
                Snippet { text: "", score: -1.0 },
                "",
                &r.source,
            ));
            entry.0 += r.score * 0.7;
            if r.score > entry.1.score {
                entry.1 = Snippet { text: &r.text, score: r.score };
            }
        }

        // 2. Process FTS results (weight 0.3)
        let fts_max = self.fts_results.iter().map(|r| r.3.abs()).fold(0.0f64, f64::max);
        for (id, title, path, bm25) in &self.fts_results {
            let norm_score = if fts_max > 0.0 { (bm25.abs() / fts_max) as f32 } else { 0.0 };
            let entry = scores.entry(id.as_str()).or_insert((
                0.0,
                Snippet { text: "", score: -1.0 },
                title.as_str(),
                path.as_str(),
            ));
            entry.0 += norm_score * 0.3;
            if entry.2.is_empty() { entry.2 = title.as_str(); }
            if entry.3.is_empty() { entry.3 = path.as_str(); }
        }

        // 3. Fill missing metadata from the supplementary metadata map
        for (topic_id, (_score, _, title, path)) in scores.iter_mut() {
            if title.is_empty() || path.is_empty() {
                if let Some((m_title, m_path)) = self.metadata.get(*topic_id) {
                    if title.is_empty() { *title = m_title.as_str(); }
                    if path.is_empty() { *path = m_path.as_str(); }
                }
            }
        }

        let mut ranked: Vec<RecallResult<'a>> = scores
            .into_iter()
            .map(|(topic_id, (score, snippet, title, file_path))| RecallResult {
                topic_id,
                title,
                file_path,
                snippet,
                score,
            })
            .collect();

        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        ranked.truncate(top_k);
        ranked
    }
}

pub async fn recall_data(
    query: &str,
    conn: &Connection,
    store: &VectorStore,
    embedder: &dyn EmbeddingProvider,
    top_k: usize,
) -> Result<RecallData> {
    let query_vec = embedder.embed(&[query.to_string()]).await?;
    let ann_results = store.search(&query_vec[0], top_k * 4).await?;

    let safe_query = query.replace('"', "\"\"");
    let fts_results = search_fts(conn, &format!("\"{}\"", safe_query), top_k * 2)
        .await
        .unwrap_or_else(|e| {
            eprintln!("warn: FTS5 search failed, falling back to ANN-only: {e}");
            vec![]
        });

    let mut metadata = HashMap::new();
    // Pre-fetch metadata for ANN-only results
    for r in &ann_results {
        // We only need to fetch if it's not in FTS results (which we don't know yet easily without a set)
        // But for simplicity, we can fetch all or do it on demand. 
        // To keep it async-friendly, we do it here.
        let mut rows = conn.query(
            "SELECT title, file_path FROM topics WHERE id=?1",
            libsql::params![r.topic_id.clone()],
        ).await?;
        if let Some(row) = rows.next().await? {
            metadata.insert(r.topic_id.clone(), (row.get(0)?, row.get(1)?));
        }
    }

    Ok(RecallData {
        ann_results,
        fts_results,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ingest::ingest_file, vector::VectorStore};
    use async_trait::async_trait;
    use std::io::Write;
    use tempfile::tempdir;

    struct FakeEmbedder(usize);
    #[async_trait]
    impl EmbeddingProvider for FakeEmbedder {
        async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            // Make "pinning" queries return a distinctive vector
            Ok(texts.iter().map(|t| {
                if t.contains("pinning") { vec![1.0f32; self.0] } else { vec![0.0f32; self.0] }
            }).collect())
        }
        fn dim(&self) -> usize { self.0 }
    }

    #[tokio::test]
    async fn test_recall_returns_relevant_topic() {
        let dir = tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("rust-pinning.md")).unwrap();
        writeln!(f, "---\ntitle: Rust Pinning\nslug: rust-pinning\n---\n\nPinning is a mechanism to prevent moves.").unwrap();

        let conn = libsql::Builder::new_local(":memory:").build().await.unwrap().connect().unwrap();
        crate::db::init_db_on_conn(&conn).await.unwrap();

        let store = VectorStore::new_with_dim(dir.path().join("lance").to_str().unwrap(), 4).await.unwrap();
        let embedder = FakeEmbedder(4);

        ingest_file(dir.path().join("rust-pinning.md").as_path(), &conn, &store, &embedder).await.unwrap();

        let data = recall_data("pinning", &conn, &store, &embedder, 5).await.unwrap();
        let results = data.ranked(5);
        assert!(!results.is_empty(), "expected at least one result");
        assert!(results[0].title.contains("Pinning"));
    }
}
