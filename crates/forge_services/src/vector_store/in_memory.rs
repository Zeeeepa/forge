//! In-memory vector store implementation for development and testing

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use forge_domain::{CodeChunk, DistanceMetric, SearchFilters, VectorStoreConfig, VectorStoreType};
use tracing::{debug, info, warn};

use super::store_trait::{IndexStatus, SearchResult, VectorStore, VectorStoreStats};

/// In-memory vector store implementation
pub struct InMemoryVectorStore {
    config: VectorStoreConfig,
    collections: RwLock<HashMap<String, Collection>>,
}

/// A collection of vectors and their metadata
#[derive(Debug, Clone)]
struct Collection {
    name: String,
    dimension: usize,
    vectors: HashMap<String, VectorEntry>,
    distance_metric: DistanceMetric,
}

/// An entry in the vector collection
#[derive(Debug, Clone)]
struct VectorEntry {
    chunk: CodeChunk,
    embedding: Vec<f32>,
    metadata: HashMap<String, String>,
}

impl Collection {
    /// Get the collection name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get collection info
    pub fn info(&self) -> String {
        format!("Collection '{}' with {} vectors, dimension {}", 
            self.name, self.vectors.len(), self.dimension)
    }
}

impl InMemoryVectorStore {
    /// Create a new in-memory vector store
    pub fn new() -> Self {
        Self {
            config: VectorStoreConfig {
                store_type: VectorStoreType::InMemory,
                collection_name: "default".to_string(),
                distance_metric: DistanceMetric::Cosine,
                enable_compression: false,
            },
            collections: RwLock::new(HashMap::new()),
        }
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Calculate euclidean distance between two vectors
    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::INFINITY;
        }

        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Calculate dot product similarity
    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Calculate manhattan distance
    fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::INFINITY;
        }

        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    }

    /// Calculate similarity score based on distance metric
    fn calculate_similarity(&self, metric: &DistanceMetric, a: &[f32], b: &[f32]) -> f32 {
        match metric {
            DistanceMetric::Cosine => Self::cosine_similarity(a, b),
            DistanceMetric::Euclidean => {
                let distance = Self::euclidean_distance(a, b);
                // Convert distance to similarity (closer to 0 is more similar)
                1.0 / (1.0 + distance)
            }
            DistanceMetric::DotProduct => Self::dot_product(a, b),
            DistanceMetric::Manhattan => {
                let distance = Self::manhattan_distance(a, b);
                // Convert distance to similarity (closer to 0 is more similar)
                1.0 / (1.0 + distance)
            }
        }
    }

    /// Check if a chunk matches the given filters
    fn matches_filters(chunk: &CodeChunk, filters: &SearchFilters) -> bool {
        // Check repository filter
        if let Some(ref repo) = filters.repository {
            if let Some(ref chunk_repo) = chunk.metadata.repository {
                if chunk_repo != repo {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check branch filter
        if let Some(ref branch) = filters.branch {
            if let Some(ref chunk_branch) = chunk.metadata.branch {
                if chunk_branch != branch {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check language filters
        if !filters.languages.is_empty()
            && !filters.languages.contains(&chunk.language) {
                return false;
            }

        // Check path filters (simple glob matching)
        if !filters.paths.is_empty() {
            let path_matches = filters.paths.iter().any(|pattern| {
                // Simple glob matching - exact match or ends with pattern
                chunk.path.contains(pattern) || chunk.path.ends_with(pattern)
            });
            if !path_matches {
                return false;
            }
        }

        // Check symbol filters
        if !filters.symbols.is_empty() {
            if let Some(ref symbol) = chunk.symbol {
                if !filters.symbols.contains(symbol) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check tags filters
        if !filters.tags.is_empty() {
            let tag_matches = filters
                .tags
                .iter()
                .any(|tag| chunk.metadata.tags.contains(tag));
            if !tag_matches {
                return false;
            }
        }

        true
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn initialize(&mut self, config: &VectorStoreConfig) -> Result<()> {
        info!("Initializing in-memory vector store");
        self.config = config.clone();
        Ok(())
    }

    async fn create_collection(&mut self, name: &str, dimension: usize) -> Result<()> {
        info!(
            "Creating collection '{}' with dimension {}",
            name, dimension
        );

        let collection = Collection {
            name: name.to_string(),
            dimension,
            vectors: HashMap::new(),
            distance_metric: self.config.distance_metric.clone(),
        };

        let mut collections = self.collections.write().unwrap();
        collections.insert(name.to_string(), collection);

        debug!("Created collection '{}' successfully", name);
        Ok(())
    }

    async fn delete_collection(&mut self, name: &str) -> Result<()> {
        info!("Deleting collection '{}'", name);

        let mut collections = self.collections.write().unwrap();
        if collections.remove(name).is_some() {
            debug!("Deleted collection '{}' successfully", name);
        } else {
            warn!("Collection '{}' not found for deletion", name);
        }

        Ok(())
    }

    async fn collection_exists(&self, name: &str) -> Result<bool> {
        let collections = self.collections.read().unwrap();
        Ok(collections.contains_key(name))
    }

    async fn insert_chunk(
        &mut self,
        collection: &str,
        chunk: &CodeChunk,
        embedding: &[f32],
    ) -> Result<String> {
        debug!(
            "Inserting chunk '{}' into collection '{}'",
            chunk.id, collection
        );

        let mut collections = self.collections.write().unwrap();
        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;

        if embedding.len() != coll.dimension {
            return Err(anyhow::anyhow!(
                "Embedding dimension {} does not match collection dimension {}",
                embedding.len(),
                coll.dimension
            ));
        }

        let entry = VectorEntry {
            chunk: chunk.clone(),
            embedding: embedding.to_vec(),
            metadata: HashMap::new(), // Could be extended to include additional metadata
        };

        coll.vectors.insert(chunk.id.clone(), entry);

        debug!("Inserted chunk '{}' successfully", chunk.id);
        Ok(chunk.id.clone())
    }

    async fn insert_chunks(
        &mut self,
        collection: &str,
        chunks: &[(CodeChunk, Vec<f32>)],
    ) -> Result<Vec<String>> {
        info!(
            "Inserting {} chunks into collection '{}'",
            chunks.len(),
            collection
        );

        let mut ids = Vec::new();
        for (chunk, embedding) in chunks {
            let id = self.insert_chunk(collection, chunk, embedding).await?;
            ids.push(id);
        }

        debug!("Inserted {} chunks successfully", chunks.len());
        Ok(ids)
    }

    async fn update_chunk(
        &mut self,
        collection: &str,
        chunk_id: &str,
        chunk: &CodeChunk,
        embedding: &[f32],
    ) -> Result<()> {
        debug!(
            "Updating chunk '{}' in collection '{}'",
            chunk_id, collection
        );

        let mut collections = self.collections.write().unwrap();
        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;

        if embedding.len() != coll.dimension {
            return Err(anyhow::anyhow!(
                "Embedding dimension {} does not match collection dimension {}",
                embedding.len(),
                coll.dimension
            ));
        }

        let entry = VectorEntry {
            chunk: chunk.clone(),
            embedding: embedding.to_vec(),
            metadata: HashMap::new(),
        };

        coll.vectors.insert(chunk_id.to_string(), entry);

        debug!("Updated chunk '{}' successfully", chunk_id);
        Ok(())
    }

    async fn delete_chunk(&mut self, collection: &str, chunk_id: &str) -> Result<()> {
        debug!(
            "Deleting chunk '{}' from collection '{}'",
            chunk_id, collection
        );

        let mut collections = self.collections.write().unwrap();
        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;

        if coll.vectors.remove(chunk_id).is_some() {
            debug!("Deleted chunk '{}' successfully", chunk_id);
        } else {
            warn!("Chunk '{}' not found for deletion", chunk_id);
        }

        Ok(())
    }

    async fn delete_chunks(&mut self, collection: &str, chunk_ids: &[String]) -> Result<()> {
        info!(
            "Deleting {} chunks from collection '{}'",
            chunk_ids.len(),
            collection
        );

        for chunk_id in chunk_ids {
            self.delete_chunk(collection, chunk_id).await?;
        }

        debug!("Deleted {} chunks successfully", chunk_ids.len());
        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_embedding: &[f32],
        limit: usize,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Searching collection '{}' with limit {}", collection, limit);

        let collections = self.collections.read().unwrap();
        let coll = collections
            .get(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;

        if query_embedding.len() != coll.dimension {
            return Err(anyhow::anyhow!(
                "Query embedding dimension {} does not match collection dimension {}",
                query_embedding.len(),
                coll.dimension
            ));
        }

        let mut results: Vec<SearchResult> = Vec::new();

        for (chunk_id, entry) in &coll.vectors {
            // Apply filters if provided
            if let Some(filters) = filters
                && !Self::matches_filters(&entry.chunk, filters) {
                    continue;
                }

            let score =
                self.calculate_similarity(&coll.distance_metric, query_embedding, &entry.embedding);

            results.push(SearchResult {
                chunk_id: chunk_id.clone(),
                chunk: entry.chunk.clone(),
                score,
                metadata: entry.metadata.clone(),
            });
        }

        // Sort by score (highest first) and limit results
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        debug!("Found {} results", results.len());
        Ok(results)
    }

    async fn get_stats(&self, collection: &str) -> Result<VectorStoreStats> {
        let collections = self.collections.read().unwrap();
        let coll = collections
            .get(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection))?;

        let stats = VectorStoreStats {
            total_vectors: coll.vectors.len(),
            vector_dimension: coll.dimension,
            storage_size_bytes: (coll.vectors.len() * coll.dimension * 4) as u64, // rough estimate
            index_status: IndexStatus::Ready,
            distance_metric: coll.distance_metric.clone(),
            additional_metrics: HashMap::new(),
        };

        Ok(stats)
    }

    fn get_config(&self) -> &VectorStoreConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use forge_domain::{ChunkMetadata, CodeChunk, SearchFilters};
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn test_create_and_delete_collection() {
        let mut fixture = InMemoryVectorStore::new();

        let actual_create = fixture.create_collection("test", 384).await;
        let actual_exists = fixture.collection_exists("test").await.unwrap();
        let actual_delete = fixture.delete_collection("test").await;
        let actual_exists_after = fixture.collection_exists("test").await.unwrap();

        assert!(actual_create.is_ok());
        assert!(actual_exists);
        assert!(actual_delete.is_ok());
        assert!(!actual_exists_after);
    }

    #[tokio::test]
    async fn test_insert_and_search_chunk() {
        let mut fixture = InMemoryVectorStore::new();
        fixture.create_collection("test", 3).await.unwrap();

        let chunk = CodeChunk::new(
            "test-1".to_string(),
            "test.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn test() {}".to_string(),
            1,
            1,
        );
        let embedding = vec![0.1, 0.2, 0.3];

        let actual_insert = fixture.insert_chunk("test", &chunk, &embedding).await;
        let actual_search = fixture
            .search("test", &[0.1, 0.2, 0.3], 10, None)
            .await
            .unwrap();

        assert!(actual_insert.is_ok());
        assert_eq!(actual_search.len(), 1);
        assert_eq!(actual_search[0].chunk_id, "test-1");
        assert!((actual_search[0].score - 1.0).abs() < 0.001); // Should be very similar
    }

    #[tokio::test]
    async fn test_search_with_filters() {
        let mut fixture = InMemoryVectorStore::new();
        fixture.create_collection("test", 3).await.unwrap();

        let mut chunk1 = CodeChunk::new(
            "test-1".to_string(),
            "test.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn test() {}".to_string(),
            1,
            1,
        );
        chunk1.metadata.repository = Some("repo1".to_string());

        let mut chunk2 = CodeChunk::new(
            "test-2".to_string(),
            "test.py".to_string(),
            "python".to_string(),
            "def456".to_string(),
            "def test(): pass".to_string(),
            1,
            1,
        );
        chunk2.metadata.repository = Some("repo2".to_string());

        let embedding = vec![0.1, 0.2, 0.3];
        fixture
            .insert_chunk("test", &chunk1, &embedding)
            .await
            .unwrap();
        fixture
            .insert_chunk("test", &chunk2, &embedding)
            .await
            .unwrap();

        let filters = SearchFilters {
            repository: Some("repo1".to_string()),
            languages: vec!["rust".to_string()],
            ..Default::default()
        };

        let actual_results = fixture
            .search("test", &embedding, 10, Some(&filters))
            .await
            .unwrap();

        assert_eq!(actual_results.len(), 1);
        assert_eq!(actual_results[0].chunk_id, "test-1");
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        let actual_same = InMemoryVectorStore::cosine_similarity(&a, &b);
        let actual_different = InMemoryVectorStore::cosine_similarity(&a, &c);

        assert!((actual_same - 1.0).abs() < 0.001);
        assert!((actual_different - 0.0).abs() < 0.001);
    }
}
