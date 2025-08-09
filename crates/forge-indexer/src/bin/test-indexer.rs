//! Simple test program to verify indexer components work correctly

use std::path::Path;

use forge_indexer::{IndexingPipeline, PipelineConfig};
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🧪 Starting indexer test");

    // Create a simple configuration
    let config = PipelineConfig::default();
    info!("🔧 Creating indexing pipeline");

    let pipeline = IndexingPipeline::new(config).await?;
    info!("✅ Pipeline created successfully");

    // Create a test file
    let test_content = r#"
    fn hello_world() {
        println!("Hello, world!");
    }
    
    fn main() {
        hello_world();
    }
    "#;

    // Write test file
    tokio::fs::write("test.rs", test_content).await?;
    info!("📄 Created test file");

    // Process the single file
    info!("🔄 Processing test file");
    let result = pipeline.process_file(Path::new("test.rs")).await;
    info!("🔄 File processing completed");

    match result {
        Ok(()) => {
            info!("✅ File processed successfully");
            let stats = pipeline.get_stats().await;
            info!(
                "📊 Stats - Files: {}, Chunks: {}, Embeddings: {}, Errors: {}",
                stats.files_processed,
                stats.chunks_created,
                stats.embeddings_generated,
                stats.errors_encountered
            );

            // Also check if the file exists
            if let Ok(content) = tokio::fs::read_to_string("test.rs").await {
                info!("📄 File content length: {}", content.len());
            }
        }
        Err(e) => {
            error!("❌ Error processing file: {}", e);
            return Err(e);
        }
    }

    // Clean up
    let _ = tokio::fs::remove_file("test.rs").await;

    info!("✅ Test completed successfully");
    Ok(())
}
