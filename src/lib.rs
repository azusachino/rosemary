//! Rosemary Personal Knowledge Base v0.2.0

// PKM Modules
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

// Async Masterclass Modules
pub mod observability;
pub mod queue;
pub mod shutdown;

#[cfg(test)]
mod tests;

pub use anyhow::{anyhow, bail, Result};
pub use tokio::task::JoinHandle;
