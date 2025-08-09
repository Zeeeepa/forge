//! Domain models for indexing operations and configuration

use std::collections::HashMap;
use std::path::PathBuf;

use derive_setters::Setters;
use serde::{Deserialize, Serialize};

use crate::code_chunk::{ChunkingConfig, IndexedCodebase};
use crate::embedding::EmbeddingConfig;

/// Configuration for indexing operations
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
#[derive(Default)]
pub struct IndexingConfig {
    /// Chunking configuration
    pub chunking: ChunkingConfig,
    /// Embedding configuration
    pub embedding: EmbeddingConfig,
    /// Vector store configuration
    pub vector_store: VectorStoreConfig,
    /// Processing configuration
    pub processing: ProcessingConfig,
    /// File filtering configuration
    pub filtering: FilterConfig,
}

/// Configuration for vector storage
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct VectorStoreConfig {
    /// Vector store type
    pub store_type: VectorStoreType,
    /// Collection/index name
    pub collection_name: String,
    /// Distance metric for similarity
    pub distance_metric: DistanceMetric,
    /// Whether to enable compression
    pub enable_compression: bool,
}

/// Available vector store types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VectorStoreType {
    /// In-memory storage (for development/testing)
    InMemory,
    /// File-based storage using serde
    FileStore {
        /// Storage directory path
        storage_path: PathBuf,
    },
    /// Qdrant vector database
    Qdrant {
        /// Server URL
        url: String,
        /// Optional API key
        api_key: Option<String>,
    },
    /// Pinecone vector database
    Pinecone {
        /// API key
        api_key: String,
        /// Environment name
        environment: String,
    },
    /// Chroma vector database
    Chroma {
        /// Server URL
        url: String,
        /// Optional authentication token
        auth_token: Option<String>,
    },
}

/// Distance metrics for vector similarity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DistanceMetric {
    /// Cosine similarity (default)
    Cosine,
    /// Euclidean distance
    Euclidean,
    /// Dot product
    DotProduct,
    /// Manhattan distance
    Manhattan,
}

/// Configuration for processing operations
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct ProcessingConfig {
    /// Maximum number of concurrent files to process
    pub max_concurrent_files: usize,
    /// Maximum number of concurrent chunks to embed
    pub max_concurrent_chunks: usize,
    /// Timeout for processing a single file (in seconds)
    pub file_timeout_seconds: u64,
    /// Whether to enable incremental indexing
    pub incremental_indexing: bool,
    /// Whether to detect file changes automatically
    pub watch_changes: bool,
    /// Retry configuration for failed operations
    pub retry_config: IndexingRetryConfig,
}

/// Configuration for retry logic
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct IndexingRetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial delay between retries (in milliseconds)
    pub initial_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Maximum delay between retries (in milliseconds)
    pub max_delay_ms: u64,
}

/// Configuration for file filtering
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct FilterConfig {
    /// Supported file extensions
    pub supported_extensions: Vec<String>,
    /// Patterns to ignore (glob patterns)
    pub ignore_patterns: Vec<String>,
    /// Maximum file size to process (in bytes)
    pub max_file_size_bytes: u64,
    /// Minimum file size to process (in bytes)
    pub min_file_size_bytes: u64,
    /// Whether to respect .gitignore files
    pub respect_gitignore: bool,
    /// Additional directories to ignore
    pub ignore_directories: Vec<String>,
}

/// Request to index a codebase
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct IndexingRequest {
    /// Unique identifier for the request
    pub request_id: String,
    /// Repository to index
    pub repository: String,
    /// Branch to index
    pub branch: String,
    /// User/author initiating the request
    pub user_id: String,
    /// Root path to index
    pub root_path: PathBuf,
    /// Configuration for indexing
    pub config: IndexingConfig,
    /// Whether to reset existing index
    pub reset_existing: bool,
    /// File patterns to include (if specified, only these files will be
    /// indexed)
    pub include_patterns: Vec<String>,
    /// Specific files to index (takes precedence over directory scanning)
    pub specific_files: Vec<PathBuf>,
}

/// Response from an indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingResponse {
    /// Request ID that generated this response
    pub request_id: String,
    /// Information about the indexed codebase
    pub codebase: IndexedCodebase,
    /// Detailed statistics about the operation
    pub statistics: IndexingStatistics,
    /// Any warnings encountered during indexing
    pub warnings: Vec<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Detailed statistics from an indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingStatistics {
    /// Total files discovered
    pub files_discovered: usize,
    /// Files successfully processed
    pub files_processed: usize,
    /// Files skipped (due to filters, errors, etc.)
    pub files_skipped: usize,
    /// Files that failed to process
    pub files_failed: usize,
    /// Total chunks created
    pub chunks_created: usize,
    /// Embeddings generated
    pub embeddings_generated: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Processing time breakdown
    pub time_breakdown: ProcessingTimeBreakdown,
    /// Error summary
    pub error_summary: HashMap<String, usize>,
    /// Language distribution
    pub language_distribution: HashMap<String, usize>,
}

/// Breakdown of processing time by stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingTimeBreakdown {
    /// Time spent discovering files
    pub file_discovery_ms: u64,
    /// Time spent reading files
    pub file_reading_ms: u64,
    /// Time spent chunking
    pub chunking_ms: u64,
    /// Time spent generating embeddings
    pub embedding_ms: u64,
    /// Time spent storing in vector database
    pub storage_ms: u64,
    /// Total processing time
    pub total_ms: u64,
}

/// Status update during indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingProgress {
    /// Request ID
    pub request_id: String,
    /// Current stage of processing
    pub stage: IndexingStage,
    /// Progress percentage (0-100)
    pub progress_percent: f32,
    /// Current file being processed
    pub current_file: Option<String>,
    /// Number of files processed so far
    pub files_processed: usize,
    /// Total files to process
    pub total_files: usize,
    /// Number of chunks created so far
    pub chunks_created: usize,
    /// Estimated time remaining (in seconds)
    pub estimated_remaining_seconds: Option<u64>,
    /// Current throughput (files per second)
    pub throughput_fps: f32,
}

/// Stages of the indexing process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexingStage {
    /// Discovering files to index
    Discovery,
    /// Reading and chunking files
    Chunking,
    /// Generating embeddings
    Embedding,
    /// Storing in vector database
    Storage,
    /// Finalizing and cleanup
    Finalization,
    /// Indexing completed
    Completed,
    /// Indexing failed
    Failed(String),
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            store_type: VectorStoreType::InMemory,
            collection_name: "codebase".to_string(),
            distance_metric: DistanceMetric::Cosine,
            enable_compression: false,
        }
    }
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_files: 10,
            max_concurrent_chunks: 50,
            file_timeout_seconds: 30,
            incremental_indexing: true,
            watch_changes: false,
            retry_config: IndexingRetryConfig::default(),
        }
    }
}

impl Default for IndexingRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 10000,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            supported_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "jsx".to_string(),
                "tsx".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "go".to_string(),
                "rb".to_string(),
                "scala".to_string(),
                "cs".to_string(),
                "php".to_string(),
                "swift".to_string(),
                "kt".to_string(),
            ],
            ignore_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/vendor/**".to_string(),
                "**/__pycache__/**".to_string(),
                "**/*.min.js".to_string(),
                "**/*.map".to_string(),
            ],
            max_file_size_bytes: 1024 * 1024, // 1MB
            min_file_size_bytes: 10,          // 10 bytes
            respect_gitignore: true,
            ignore_directories: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "vendor".to_string(),
                "__pycache__".to_string(),
                ".fastembed_cache".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::embedding::EmbeddingProvider;

    #[test]
    fn test_default_indexing_config() {
        let fixture = IndexingConfig::default();

        let actual_provider = &fixture.embedding.provider;
        let expected_provider = &EmbeddingProvider::Mock { dimension: 384 };

        assert_eq!(actual_provider, expected_provider);
    }

    #[test]
    fn test_indexing_request_builder() {
        let fixture = IndexingRequest {
            request_id: "test-123".to_string(),
            repository: "my-repo".to_string(),
            branch: "main".to_string(),
            user_id: "user-456".to_string(),
            root_path: PathBuf::from("/path/to/repo"),
            config: IndexingConfig::default(),
            reset_existing: false,
            include_patterns: vec![],
            specific_files: vec![],
        };

        let actual_repo = fixture.repository.clone();
        let expected_repo = "my-repo".to_string();

        assert_eq!(actual_repo, expected_repo);
    }

    #[test]
    fn test_vector_store_types() {
        let fixture_memory = VectorStoreType::InMemory;
        let fixture_qdrant =
            VectorStoreType::Qdrant { url: "http://localhost:6334".to_string(), api_key: None };

        let actual_memory = fixture_memory;
        let actual_qdrant = fixture_qdrant;

        assert_eq!(actual_memory, VectorStoreType::InMemory);
        assert!(matches!(actual_qdrant, VectorStoreType::Qdrant { .. }));
    }

    #[test]
    fn test_retry_config_defaults() {
        let fixture = IndexingRetryConfig::default();

        let actual_attempts = fixture.max_attempts;
        let actual_delay = fixture.initial_delay_ms;

        let expected_attempts = 3;
        let expected_delay = 1000;

        assert_eq!(actual_attempts, expected_attempts);
        assert_eq!(actual_delay, expected_delay);
    }
}
