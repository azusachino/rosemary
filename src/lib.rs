//! Rosemary Personal Knowledge Base v0.2.0

pub mod db;
pub mod kb;
pub mod embed;
pub mod chunk;
pub mod vector;
pub mod ingest;
pub mod recall;
pub mod digest;
pub mod compact;
pub mod tui;

pub use anyhow::{anyhow, bail, Result};
