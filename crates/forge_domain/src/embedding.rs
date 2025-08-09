//! Domain interfaces and types for text embedding

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use derive_setters::Setters;
use serde::{Deserialize, Serialize};

/// Supported embedding providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmbeddingProvider {
    /// OpenAI embedding API
    OpenAI { model: String, api_key: String },
    /// Local model using ONNX Runtime
    Local {
        model_path: PathBuf,
        tokenizer_path: PathBuf,
    },
    /// Mock embedder for testing
    Mock { dimension: usize },
}

/// Configuration for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct EmbeddingConfig {
    /// Provider to use for embeddings
    pub provider: EmbeddingProvider,
    /// Batch size for embedding generation
    pub batch_size: usize,
    /// Maximum text length before truncation
    pub max_text_length: usize,
    /// Whether to preprocess code for better embeddings
    pub preprocess_code: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::Mock { dimension: 384 },
            batch_size: 32,
            max_text_length: 8000,
            preprocess_code: true,
        }
    }
}

/// Trait for generating embeddings from text
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate embedding for a single text
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts in batch
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get the dimension of embeddings produced by this embedder
    fn embedding_dimension(&self) -> usize;

    /// Get the provider configuration
    fn provider(&self) -> &EmbeddingProvider;

    /// Get the name/identifier of this embedder
    fn name(&self) -> &str;
}

/// Utility functions for text preprocessing
pub mod preprocessing {
    /// Preprocess code text for better embedding quality
    pub fn preprocess_code_for_embedding(text: &str) -> String {
        let mut processed = text.to_string();

        // Remove excessive whitespace while preserving structure
        processed = processed
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Add semantic markers for better understanding
        let mut enhanced = String::new();

        // Add language context hints
        if processed.contains("fn ") || processed.contains("impl ") {
            enhanced.push_str("RUST_CODE: ");
        } else if processed.contains("def ") || processed.contains("class ") {
            enhanced.push_str("PYTHON_CODE: ");
        } else if processed.contains("function ") || processed.contains("const ") {
            enhanced.push_str("JAVASCRIPT_CODE: ");
        }

        // Add function/class markers
        if processed.contains("fn ")
            || processed.contains("def ")
            || processed.contains("function ")
        {
            enhanced.push_str("FUNCTION_DEFINITION ");
        }
        if processed.contains("struct ")
            || processed.contains("class ")
            || processed.contains("interface ")
        {
            enhanced.push_str("TYPE_DEFINITION ");
        }
        if processed.contains("impl ") || processed.contains("trait ") {
            enhanced.push_str("IMPLEMENTATION ");
        }
        if processed.contains("test") || processed.contains("#[test]") {
            enhanced.push_str("TEST_CODE ");
        }

        enhanced.push_str(&processed);

        // Limit length to avoid token limits
        if enhanced.len() > 8000 {
            enhanced.truncate(8000);
            enhanced.push_str("...[TRUNCATED]");
        }

        enhanced
    }

    /// Generate deterministic hash-based embedding
    pub fn generate_hash_embedding(text: &str, dimension: usize, seed: Option<u64>) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        if let Some(seed_val) = seed {
            seed_val.hash(&mut hasher);
        }
        let hash = hasher.finish();

        // Generate embedding vector from hash
        let mut embedding = Vec::with_capacity(dimension);
        let mut current_seed = hash;

        for _ in 0..dimension {
            // Simple linear congruential generator for deterministic values
            current_seed = current_seed.wrapping_mul(1103515245).wrapping_add(12345);
            let value = ((current_seed >> 16) as f32) / 65536.0 - 0.5; // Range: [-0.5, 0.5]
            embedding.push(value);
        }

        // Normalize the embedding
        normalize_vector(&mut embedding);
        embedding
    }

    /// Normalize a vector to unit length
    pub fn normalize_vector(vector: &mut [f32]) {
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vector.iter_mut().for_each(|x| *x /= norm);
        }
    }

    /// Check if a vector is valid (no NaN, infinity, etc.)
    pub fn is_valid_vector(vector: &[f32]) -> bool {
        vector.iter().all(|&x| x.is_finite())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::preprocessing::*;

    #[test]
    fn test_preprocess_code_for_embedding() {
        let fixture = "fn main() {\n    println!(\"Hello\");\n}";
        let actual = preprocess_code_for_embedding(fixture);
        let expected = "RUST_CODE: FUNCTION_DEFINITION fn main() {\nprintln!(\"Hello\");\n}";
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_generate_hash_embedding() {
        let text = "test content";
        let dimension = 10;

        let actual1 = generate_hash_embedding(text, dimension, None);
        let actual2 = generate_hash_embedding(text, dimension, None);

        assert_eq!(actual1, actual2); // Should be deterministic
        assert_eq!(actual1.len(), dimension);

        // Check that vector is normalized
        let magnitude: f32 = actual1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_vector() {
        let mut fixture = vec![3.0, 4.0]; // 3-4-5 triangle
        normalize_vector(&mut fixture);

        let expected = vec![0.6, 0.8];
        let actual = fixture;

        // Check each component with tolerance
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-6);
        }
    }

    #[test]
    fn test_is_valid_vector() {
        let valid_vector = vec![1.0, 2.0, 3.0];
        let invalid_vector = vec![1.0, f32::NAN, 3.0];

        assert!(is_valid_vector(&valid_vector));
        assert!(!is_valid_vector(&invalid_vector));
    }
}
