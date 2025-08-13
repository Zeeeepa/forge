use std::path::Path;

use anyhow::Result;
use forge_indexer::IndexingPipeline;
use tracing::{error, info};

use super::cli::IndexArgs;
use super::config::load_config_from_args;
use super::signals::setup_shutdown_signal;

/// Run the indexer service
pub async fn run_indexer(args: IndexArgs) -> Result<()> {
    info!(
        "⚙️  Configuration loaded - Path: {}, Embedder: {}, Batch size: {}, Max concurrent: {}",
        args.path, args.embedder, args.batch_size, args.max_concurrent_files
    );

    // Load configuration from command line arguments
    let config = match load_config_from_args(&args) {
        Ok(config) => {
            info!("✅ Configuration validated successfully");
            config
        }
        Err(e) => {
            error!("❌ Configuration validation failed: {}", e);
            return Err(e);
        }
    };

    // Initialize the indexing pipeline
    info!("🔧 Initializing indexing pipeline...");
    // Increase walker limits to process more files
    let walker = forge_walker::Walker::min_all()
        .max_files(10000)
        .max_depth(1024)  // Increase from default 100
        .max_total_size(100 * 1024 * 1024)  // Increase from default 10MB
        .max_file_size(10 * 1024 * 1024);  // Increase from default 1MB

    let mut pipeline = match IndexingPipeline::new_with_walker(config, walker).await {
        Ok(pipeline) => {
            info!("✅ IndexingPipeline initialized successfully");
            pipeline
        }
        Err(e) => {
            error!("❌ Failed to initialize IndexingPipeline: {}", e);
            return Err(e);
        }
    };

    // Get the directory to watch from command line
    let watch_path = Path::new(&args.path).to_path_buf();
    if !watch_path.exists() {
        error!("❌ Watch path does not exist: {:?}", watch_path);
        return Err(anyhow::anyhow!(
            "Watch path does not exist: {:?}",
            watch_path
        ));
    }
    info!("👀 Setting up file watcher for directory: {:?}", watch_path);

    // Start watching for file changes
    match pipeline.start_watching(&watch_path).await {
        Ok(()) => {
            info!("✅ File watcher started successfully");
        }
        Err(e) => {
            error!("❌ Failed to start file watcher: {}", e);
            return Err(e);
        }
    }

    // Process initial files in the directory
    info!("🔍 Processing initial files in directory...");
    let initial_files = match pipeline.collect_files_from_directory(&watch_path).await {
        Ok(files) => files,
        Err(e) => {
            error!("❌ Failed to collect files from directory: {}", e);
            return Err(e);
        }
    };

    if !initial_files.is_empty() {
        info!("📄 Found {} initial files to process", initial_files.len());
        if let Err(e) = pipeline.process_files(initial_files).await {
            error!("❌ Error processing initial files: {}", e);
            return Err(e);
        }
    } else {
        info!("📂 No initial files found in directory");
    }

    // Set up graceful shutdown
    info!("🛡️  Setting up graceful shutdown handlers");
    let shutdown_signal = setup_shutdown_signal();

    info!("🔄 Starting event processing loop...");
    // Process events until shutdown
    tokio::select! {
        result = pipeline.process_events() => {
            match result {
                Ok(()) => info!("✅ Event processing completed normally"),
                Err(e) => error!("❌ Event processing error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("🛑 Shutdown signal received, initiating graceful shutdown...");
        }
    }

    // Print final statistics
    info!("📊 Collecting final statistics...");
    let stats = pipeline.get_stats().await;
    info!("📈 Final Statistics:");
    info!("   📁 Files processed: {}", stats.files_processed);
    info!("   🧩 Chunks created: {}", stats.chunks_created);
    info!("   🤖 Embeddings generated: {}", stats.embeddings_generated);
    info!(
        "   💾 Bytes processed: {} ({:.2} MB)",
        stats.bytes_processed,
        stats.bytes_processed as f64 / 1_048_576.0
    );
    info!("   ❌ Errors encountered: {}", stats.errors_encountered);

    info!("👋 Forge Indexer shutdown complete");
    Ok(())
}
