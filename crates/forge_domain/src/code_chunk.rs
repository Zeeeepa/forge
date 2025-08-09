//! Domain models for code chunks and indexing

use std::collections::HashMap;

use derive_setters::Setters;
use serde::{Deserialize, Serialize};

/// Represents a piece of code with metadata for indexing and search
#[derive(Debug, Clone, Serialize, Deserialize, Setters, PartialEq)]
#[setters(strip_option, into)]
pub struct CodeChunk {
    /// Unique identifier for the chunk
    pub id: String,
    /// File path relative to repository root
    pub path: String,
    /// Programming language detected for the file
    pub language: String,
    /// Symbol context (function, class, module name)
    pub symbol: Option<String>,
    /// Git revision/commit hash when indexed
    pub revision: String,
    /// Size of the chunk in characters
    pub size: usize,
    /// Actual code content
    pub content: String,
    /// Optional semantic summary of the chunk
    pub summary: Option<String>,
    /// Vector embedding (not persisted in search results)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Start line number in the original file
    pub start_line: usize,
    /// End line number in the original file
    pub end_line: usize,
    /// Additional metadata for filtering and context
    pub metadata: ChunkMetadata,
}

/// Metadata associated with a code chunk
#[derive(Debug, Clone, Serialize, Deserialize, Setters, PartialEq, Default)]
#[setters(strip_option, into)]
pub struct ChunkMetadata {
    /// Repository name
    pub repository: Option<String>,
    /// Branch name
    pub branch: Option<String>,
    /// User/author identifier
    pub author: Option<String>,
    /// File modification timestamp
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Complexity score (calculated)
    pub complexity_score: Option<f32>,
    /// Dependencies/imports found in chunk
    pub dependencies: Vec<String>,
    /// Keywords extracted from chunk
    pub keywords: Vec<String>,
    /// Additional tags for categorization
    pub tags: Vec<String>,
}

/// Configuration for code chunking strategies
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct ChunkingConfig {
    /// Maximum size of a chunk in characters
    pub max_chunk_size: usize,
    /// Minimum size of a chunk in characters  
    pub min_chunk_size: usize,
    /// Overlap size between adjacent chunks
    pub overlap_size: usize,
    /// Strategy to use for chunking
    pub strategy: ChunkingStrategy,
    /// Languages to enable semantic chunking for
    pub semantic_languages: Vec<String>,
}

/// Available chunking strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkingStrategy {
    /// Semantic chunking using AST parsing (preferred)
    Semantic,
    /// Size-based chunking with overlap (fallback)
    SizeBased,
    /// Hybrid approach combining both strategies
    Hybrid,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 1000,
            min_chunk_size: 100,
            overlap_size: 100,
            strategy: ChunkingStrategy::Hybrid,
            semantic_languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "java".to_string(),
                "go".to_string(),
                "cpp".to_string(),
                "c".to_string(),
            ],
        }
    }
}

/// Collection of code chunks representing an indexed codebase
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct IndexedCodebase {
    /// Unique identifier for the indexed codebase
    pub id: String,
    /// Repository information
    pub repository: String,
    /// Branch that was indexed
    pub branch: String,
    /// Total number of files indexed
    pub files_count: usize,
    /// Total number of chunks created
    pub chunks_count: usize,
    /// Timestamp when indexing was completed
    pub indexed_at: chrono::DateTime<chrono::Utc>,
    /// Indexing configuration used
    pub config: ChunkingConfig,
    /// Languages detected and their file counts
    pub language_stats: HashMap<String, usize>,
    /// Total size in bytes of indexed content
    pub total_size_bytes: u64,
    /// Status of the indexing process
    pub status: IndexingStatus,
}

/// Status of an indexing operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexingStatus {
    /// Indexing is in progress
    InProgress,
    /// Indexing completed successfully
    Completed,
    /// Indexing failed with error
    Failed(String),
    /// Indexing was cancelled
    Cancelled,
}

impl CodeChunk {
    /// Create a new code chunk with required fields
    pub fn new(
        id: String,
        path: String,
        language: String,
        revision: String,
        content: String,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        let size = content.len();
        Self {
            id,
            path,
            language,
            symbol: None,
            revision,
            size,
            content,
            summary: None,
            embedding: None,
            start_line,
            end_line,
            metadata: ChunkMetadata::default(),
        }
    }

    /// Calculate a content hash for the chunk (useful for deduplication)
    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.content);
        hasher.update(&self.path);
        hasher.update(&self.language);
        format!("{:x}", hasher.finalize())
    }

    /// Get a display name for the chunk (path:symbol or path:lines)
    pub fn display_name(&self) -> String {
        match &self.symbol {
            Some(symbol) => format!("{}:{}", self.path, symbol),
            None => format!("{}:{}-{}", self.path, self.start_line, self.end_line),
        }
    }

    /// Check if chunk is from a specific language
    pub fn is_language(&self, language: &str) -> bool {
        self.language.eq_ignore_ascii_case(language)
    }

    /// Get the line range as a tuple
    pub fn line_range(&self) -> (usize, usize) {
        (self.start_line, self.end_line)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_code_chunk_creation() {
        let fixture = CodeChunk::new(
            "test-id".to_string(),
            "src/main.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn main() {}".to_string(),
            1,
            1,
        );

        let actual = fixture.clone();
        let expected = CodeChunk {
            id: "test-id".to_string(),
            path: "src/main.rs".to_string(),
            language: "rust".to_string(),
            symbol: None,
            revision: "abc123".to_string(),
            size: 12,
            content: "fn main() {}".to_string(),
            summary: None,
            embedding: None,
            start_line: 1,
            end_line: 1,
            metadata: ChunkMetadata::default(),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_display_name_with_symbol() {
        let fixture = CodeChunk::new(
            "test-id".to_string(),
            "src/lib.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn test() {}".to_string(),
            10,
            15,
        )
        .symbol("test".to_string());

        let actual = fixture.display_name();
        let expected = "src/lib.rs:test";

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_display_name_without_symbol() {
        let fixture = CodeChunk::new(
            "test-id".to_string(),
            "src/lib.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn test() {}".to_string(),
            10,
            15,
        );

        let actual = fixture.display_name();
        let expected = "src/lib.rs:10-15";

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_content_hash() {
        let fixture = CodeChunk::new(
            "test-id".to_string(),
            "src/main.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn main() {}".to_string(),
            1,
            1,
        );

        let actual = fixture.content_hash();

        // Hash should be deterministic
        assert!(!actual.is_empty());
        assert_eq!(actual.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_language_check() {
        let fixture = CodeChunk::new(
            "test-id".to_string(),
            "src/main.rs".to_string(),
            "rust".to_string(),
            "abc123".to_string(),
            "fn main() {}".to_string(),
            1,
            1,
        );

        let actual_rust = fixture.is_language("rust");
        let actual_python = fixture.is_language("python");
        let actual_rust_case = fixture.is_language("RUST");

        assert!(actual_rust);
        assert!(!actual_python);
        assert!(actual_rust_case);
    }
}
