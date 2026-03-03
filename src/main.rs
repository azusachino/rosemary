use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "rosemary")]
#[command(about = "Personal Knowledge Base CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest a new topic and content
    Ingest {
        topic: String,
        content: String,
    },
    /// Recall topics or gists using semantic search
    Recall {
        query: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { topic, content: _ } => {
            println!("Ingesting topic: {}...", topic);
        }
        Commands::Recall { query } => {
            println!("Recalling: {}...", query);
        }
    }

    Ok(())
}
