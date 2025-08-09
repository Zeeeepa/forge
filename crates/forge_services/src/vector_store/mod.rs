//! Vector store implementations and utilities

mod in_memory;
mod store_trait;

use std::sync::Arc;

use anyhow::Result;
use forge_domain::{VectorStoreConfig, VectorStoreType};
pub use in_memory::InMemoryVectorStore;
pub use store_trait::{IndexStatus, SearchResult, VectorStore, VectorStoreStats};

// Re-export utility functions from domain
pub use forge_domain::preprocessing::{normalize_vector, is_valid_vector};

/// Factory for creating vector store instances
pub struct VectorStoreFactory;

impl VectorStoreFactory {
    /// Create a vector store instance based on configuration
    pub async fn create(config: &VectorStoreConfig) -> Result<Box<dyn VectorStore>> {
        let mut store: Box<dyn VectorStore> = match &config.store_type {
            VectorStoreType::InMemory => Box::new(InMemoryVectorStore::new()),
            VectorStoreType::FileStore { storage_path: _ } => {
                // TODO: Implement file-based vector store
                return Err(anyhow::anyhow!(
                    "File-based vector store not yet implemented"
                ));
            }
            VectorStoreType::Qdrant { url: _, api_key: _ } => {
                // TODO: Implement Qdrant vector store
                return Err(anyhow::anyhow!("Qdrant vector store not yet implemented"));
            }
            VectorStoreType::Pinecone { api_key: _, environment: _ } => {
                // TODO: Implement Pinecone vector store
                return Err(anyhow::anyhow!("Pinecone vector store not yet implemented"));
            }
            VectorStoreType::Chroma { url: _, auth_token: _ } => {
                // TODO: Implement Chroma vector store
                return Err(anyhow::anyhow!("Chroma vector store not yet implemented"));
            }
        };

        store.initialize(config).await?;
        Ok(store)
    }

    /// Create an in-memory vector store for testing
    pub fn create_in_memory() -> Box<dyn VectorStore> {
        Box::new(InMemoryVectorStore::new())
    }
}

/// Shared vector store wrapped in Arc for thread safety
pub type SharedVectorStore = Arc<tokio::sync::RwLock<Box<dyn VectorStore>>>;

#[cfg(test)]
mod tests {
    use forge_domain::{DistanceMetric, VectorStoreConfig, VectorStoreType};
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn test_vector_store_factory_in_memory() {
        let config = VectorStoreConfig {
            store_type: VectorStoreType::InMemory,
            collection_name: "test".to_string(),
            distance_metric: DistanceMetric::Cosine,
            enable_compression: false,
        };

        let actual = VectorStoreFactory::create(&config).await;

        assert!(actual.is_ok());
    }

    #[test]
    fn test_normalize_vector() {
        let mut fixture = vec![3.0, 4.0];
        normalize_vector(&mut fixture);

        let actual_magnitude = fixture.iter().map(|x| x * x).sum::<f32>().sqrt();
        let expected_magnitude = 1.0;

        assert!((actual_magnitude - expected_magnitude).abs() < 0.001);
    }

    #[test]
    fn test_is_valid_vector() {
        let fixture_valid = vec![1.0, 2.0, 3.0];
        let fixture_invalid = vec![1.0, f32::NAN, 3.0];

        let actual_valid = is_valid_vector(&fixture_valid);
        let actual_invalid = is_valid_vector(&fixture_invalid);

        assert!(actual_valid);
        assert!(!actual_invalid);
    }
}
