//! Vector store abstraction and implementations

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use forge_domain::{CodeChunk, DistanceMetric, SearchFilters, VectorStoreConfig};
use serde::{Deserialize, Serialize};

/// Trait for vector store implementations
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Initialize the vector store with configuration
    async fn initialize(&mut self, config: &VectorStoreConfig) -> Result<()>;

    /// Create a collection/index for storing vectors
    async fn create_collection(&mut self, name: &str, dimension: usize) -> Result<()>;

    /// Delete a collection/index
    async fn delete_collection(&mut self, name: &str) -> Result<()>;

    /// Check if a collection exists
    async fn collection_exists(&self, name: &str) -> Result<bool>;

    /// Insert a single chunk with its embedding
    async fn insert_chunk(
        &mut self,
        collection: &str,
        chunk: &CodeChunk,
        embedding: &[f32],
    ) -> Result<String>;

    /// Insert multiple chunks with their embeddings
    async fn insert_chunks(
        &mut self,
        collection: &str,
        chunks: &[(CodeChunk, Vec<f32>)],
    ) -> Result<Vec<String>>;

    /// Update an existing chunk
    async fn update_chunk(
        &mut self,
        collection: &str,
        chunk_id: &str,
        chunk: &CodeChunk,
        embedding: &[f32],
    ) -> Result<()>;

    /// Delete a chunk by ID
    async fn delete_chunk(&mut self, collection: &str, chunk_id: &str) -> Result<()>;

    /// Delete multiple chunks by IDs
    async fn delete_chunks(&mut self, collection: &str, chunk_ids: &[String]) -> Result<()>;

    /// Search for similar vectors
    async fn search(
        &self,
        collection: &str,
        query_embedding: &[f32],
        limit: usize,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>>;

    /// Get statistics about the vector store
    async fn get_stats(&self, collection: &str) -> Result<VectorStoreStats>;

    /// Get the configuration of the vector store
    fn get_config(&self) -> &VectorStoreConfig;
}

/// Result from a vector search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The chunk ID
    pub chunk_id: String,
    /// The matching chunk data
    pub chunk: CodeChunk,
    /// Similarity score (0.0 to 1.0, higher is more similar)
    pub score: f32,
    /// Additional metadata from the vector store
    pub metadata: HashMap<String, String>,
}

/// Statistics about a vector store collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    /// Number of vectors/chunks in the collection
    pub total_vectors: usize,
    /// Dimension of vectors
    pub vector_dimension: usize,
    /// Total storage size in bytes
    pub storage_size_bytes: u64,
    /// Index build status
    pub index_status: IndexStatus,
    /// Distance metric being used
    pub distance_metric: DistanceMetric,
    /// Additional store-specific metrics
    pub additional_metrics: HashMap<String, f64>,
}

/// Status of the vector index
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexStatus {
    /// Index is building
    Building,
    /// Index is ready for queries
    Ready,
    /// Index is corrupted or has errors
    Error(String),
}

#[cfg(test)]
mod tests {
    use forge_domain::{ChunkMetadata, CodeChunk};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_search_result_creation() {
        let fixture_chunk = CodeChunk {
            id: "test-chunk".to_string(),
            path: "test.rs".to_string(),
            language: "rust".to_string(),
            symbol: Some("test_fn".to_string()),
            revision: "abc123".to_string(),
            size: 100,
            content: "fn test() {}".to_string(),
            summary: None,
            embedding: None,
            start_line: 1,
            end_line: 3,
            metadata: ChunkMetadata::default(),
        };

        let fixture = SearchResult {
            chunk_id: "test-chunk".to_string(),
            chunk: fixture_chunk.clone(),
            score: 0.95,
            metadata: HashMap::new(),
        };

        let actual = fixture.chunk.id.clone();
        let expected = "test-chunk".to_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_vector_store_stats() {
        let fixture = VectorStoreStats {
            total_vectors: 1000,
            vector_dimension: 1536,
            storage_size_bytes: 1024 * 1024,
            index_status: IndexStatus::Ready,
            distance_metric: DistanceMetric::Cosine,
            additional_metrics: HashMap::new(),
        };

        let actual_vectors = fixture.total_vectors;
        let actual_status = fixture.index_status;

        let expected_vectors = 1000;
        let expected_status = IndexStatus::Ready;

        assert_eq!(actual_vectors, expected_vectors);
        assert_eq!(actual_status, expected_status);
    }
}
