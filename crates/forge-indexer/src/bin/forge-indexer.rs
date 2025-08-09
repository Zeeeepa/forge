//! Production-grade indexer service with end-to-end pipeline

use anyhow::Result;
use clap::Parser;
use tracing::info;

mod indexer;

use indexer::{Args, Commands, run_indexer, run_reset};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Forge Indexer v2.0 - Production Grade");
    info!(
        "📋 System information: OS={}, Architecture={}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // Parse command line arguments
    let args = Args::parse();

    match args.command {
        Commands::Index {
            path,
            embedder,
            openai_api_key,
            local_model_path,
            local_tokenizer_path,
            batch_size,
            max_concurrent_files,
        } => {
            run_indexer(indexer::IndexArgs {
                path,
                embedder,
                openai_api_key,
                local_model_path,
                local_tokenizer_path,
                batch_size,
                max_concurrent_files,
            })
            .await
        }
        Commands::Reset { embedder } => run_reset(embedder).await,
    }
}
