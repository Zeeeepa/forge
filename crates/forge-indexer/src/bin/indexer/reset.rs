use std::sync::Arc;

use anyhow::Result;
use forge_indexer::IndexService;
use forge_indexer::embedder::{Embedder, LocalEmbedder, OpenAIEmbedder};
use tracing::info;

/// Run the reset command to clear the Qdrant collection
pub async fn run_reset(embedder_type: String) -> Result<()> {
    info!("🔄 Starting Qdrant collection reset...");
    info!("📏 Embedder type: {}", embedder_type);

    // Create embedder to get dimensions
    let embedder: Arc<dyn Embedder> = match embedder_type.as_str() {
        "openai" => {
            // Check if API key is available in environment or fail gracefully
            if std::env::var("OPENAI_API_KEY").is_err() {
                return Err(anyhow::anyhow!("OPENAI_API_KEY environment variable required for OpenAI embedder"));
            }
            Arc::new(OpenAIEmbedder::new().await?)
        }
        "local" => Arc::new(LocalEmbedder::new_default()?),
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid embedder type: {}. Must be openai or local",
                embedder_type
            ));
        }
    };

    let vector_dimension = embedder.embedding_dimension();
    info!("📐 Vector dimension: {}", vector_dimension);

    // Create index service and reset collection
    let index_service = IndexService::new(vector_dimension).await?;

    info!("🗑️  Resetting Qdrant collection...");
    index_service.reset_collection().await?;

    info!("✅ Qdrant collection reset completed successfully!");
    info!("💡 You can now restart the indexer to rebuild the index with the new embedding model.");

    Ok(())
}
