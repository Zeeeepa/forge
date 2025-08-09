//! Indexing services and utilities

mod indexing_service;
mod mock_embedder;
mod simple_chunker;

use anyhow::Result;
use forge_domain::{IndexingConfig, Chunker, Embedder};
pub use indexing_service::IndexingService;
pub use mock_embedder::MockEmbedder;
pub use simple_chunker::SimpleChunker;

// Re-export utility functions
pub use utils::{supports_semantic_chunking, recommended_chunk_size};

/// Factory for creating indexing service instances
pub struct IndexingServiceFactory;

impl IndexingServiceFactory {
    /// Create an indexing service with default implementations
    pub async fn create_default(_config: IndexingConfig) -> Result<IndexingService> {
        // Create default implementations
        let chunker = Box::new(SimpleChunker::new());
        let embedder = Box::new(MockEmbedder::new(384));

        // Create the service
        IndexingService::new(_config, chunker, embedder).await
    }

    /// Create an indexing service with custom implementations
    pub async fn create_with_implementations(
        config: IndexingConfig,
        chunker: Box<dyn Chunker>,
        embedder: Box<dyn Embedder>,
    ) -> Result<IndexingService> {
        IndexingService::new(config, chunker, embedder).await
    }
}

/// Utility functions for indexing operations
pub mod utils {
    /// Check if a file extension is supported for semantic chunking
    pub fn supports_semantic_chunking(language: &str) -> bool {
        matches!(
            language,
            "rust"
                | "python"
                | "javascript"
                | "typescript"
                | "java"
                | "cpp"
                | "c"
                | "go"
                | "ruby"
                | "scala"
        )
    }

    /// Get recommended chunk size for a language
    pub fn recommended_chunk_size(language: &str) -> usize {
        match language {
            // Dense languages with significant meaning per line
            "rust" | "scala" | "haskell" => 800,
            // Standard object-oriented languages
            "java" | "cpp" | "csharp" => 1000,
            // Dynamic languages
            "python" | "ruby" | "javascript" | "typescript" => 1200,
            // Simple/configuration languages
            "json" | "yaml" | "toml" | "css" => 600,
            // Documentation
            "markdown" | "rst" | "text" => 1500,
            // Default
            _ => 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_supports_semantic_chunking() {
        let fixtures = vec![
            ("rust", true),
            ("python", true),
            ("javascript", true),
            ("text", false),
            ("json", false),
        ];

        for (language, expected) in fixtures {
            let actual = utils::supports_semantic_chunking(language);
            assert_eq!(actual, expected, "Language: {}", language);
        }
    }

    #[test]
    fn test_recommended_chunk_size() {
        let fixtures = vec![
            ("rust", 800),
            ("java", 1000),
            ("python", 1200),
            ("json", 600),
            ("markdown", 1500),
            ("unknown", 1000),
        ];

        for (language, expected) in fixtures {
            let actual = utils::recommended_chunk_size(language);
            assert_eq!(actual, expected, "Language: {}", language);
        }
    }
}
