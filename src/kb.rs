use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use slug::slugify;
use chrono::Utc;

pub fn save_markdown(topic: &str, content: &str) -> Result<PathBuf> {
    let slug = slugify(topic);
    let dir = PathBuf::from("kb/topics");
    fs::create_dir_all(&dir)?;
    
    let file_path = dir.join(format!("{}.md", slug));
    let full_content = format!(
        "---
title: {}
slug: {}
created_at: {}
---

{}",
        topic,
        slug,
        Utc::now().to_rfc3339(),
        content
    );
    
    fs::write(&file_path, full_content)?;
    Ok(file_path)
}
