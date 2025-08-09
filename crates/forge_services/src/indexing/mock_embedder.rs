//! Mock embedder implementation for testing

use anyhow::Result;
use async_trait::async_trait;
use forge_domain::{EmbeddingProvider, Embedder, preprocessing::{generate_hash_embedding, is_valid_vector, normalize_vector}};

/// A mock embedder that generates deterministic embeddings for testing
pub struct MockEmbedder {
    dimension: usize,
    provider: EmbeddingProvider,
}

impl MockEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            provider: EmbeddingProvider::Mock { dimension },
        }
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Use the shared hash-based embedding generation
        let mut embedding = generate_hash_embedding(text, self.dimension, None);

        // Validate the generated vector
        if !is_valid_vector(&embedding) {
            return Err(anyhow::anyhow!("Generated invalid vector"));
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed_text(text).await?);
        }
        Ok(embeddings)
    }

    fn embedding_dimension(&self) -> usize {
        self.dimension
    }

    fn provider(&self) -> &EmbeddingProvider {
        &self.provider
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn test_mock_embedder_deterministic() {
        let embedder = MockEmbedder::new(128);
        let text = "Hello, world!";

        let embedding1 = embedder.embed_text(text).await.unwrap();
        let embedding2 = embedder.embed_text(text).await.unwrap();

        assert_eq!(embedding1, embedding2);
        assert_eq!(embedding1.len(), 128);
    }

    #[tokio::test]
    async fn test_mock_embedder_different_texts() {
        let embedder = MockEmbedder::new(64);

        let embedding1 = embedder.embed_text("text1").await.unwrap();
        let embedding2 = embedder.embed_text("text2").await.unwrap();

        assert_ne!(embedding1, embedding2);
        assert_eq!(embedding1.len(), 64);
        assert_eq!(embedding2.len(), 64);
    }

    #[tokio::test]
    async fn test_mock_embedder_batch() {
        let embedder = MockEmbedder::new(32);
        let texts = vec!["text1".to_string(), "text2".to_string()];

        let actual = embedder.embed_batch(&texts).await.unwrap();
        let expected = vec![
            embedder.embed_text("text1").await.unwrap(),
            embedder.embed_text("text2").await.unwrap(),
        ];

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_vector_normalization() {
        let embedder = MockEmbedder::new(3);
        let embedding = embedder.embed_text("test").await.unwrap();

        // Check that the vector is normalized (magnitude should be ~1.0)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-6,
            "Vector should be normalized, got magnitude: {}",
            magnitude
        );
    }

    #[test]
    fn test_embedding_dimension() {
        let embedder = MockEmbedder::new(256);
        assert_eq!(embedder.embedding_dimension(), 256);
    }

    #[test]
    fn test_provider() {
        let embedder = MockEmbedder::new(128);
        let provider = embedder.provider();
        assert!(matches!(provider, EmbeddingProvider::Local { .. }));
    }
}
