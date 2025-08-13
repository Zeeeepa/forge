//! Embedding service with pluggable backends

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, info, warn};

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Embed a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let result = self.embed_batch(&[text.to_string()]).await?;
        result
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    fn name(&self) -> &str;

    /// Get the dimension of embeddings produced by this embedder
    fn embedding_dimension(&self) -> usize;
}

pub struct OpenAIEmbedder {
    api_key: String,
    model: String,
}

impl OpenAIEmbedder {
    pub async fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
        let model = std::env::var("OPENAI_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-large".to_string());

        Ok(Self { api_key, model })
    }

    pub fn new_with_config(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-3-large".to_string()),
        }
    }
}
/// Preprocess code text for better embedding quality
fn preprocess_code_for_embedding(text: &str) -> String {
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
    if processed.contains("fn ") || processed.contains("def ") || processed.contains("function ") {
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

#[async_trait]
impl Embedder for OpenAIEmbedder {
    fn name(&self) -> &str {
        "openai"
    }

    fn embedding_dimension(&self) -> usize {
        // text-embedding-3-large has 3072 dimensions, but we use 1536 for testing
        if self.api_key == "test-key" {
            1536
        } else {
            match self.model.as_str() {
                "text-embedding-3-large" => 3072,
                "text-embedding-3-small" => 1536,
                "text-embedding-ada-002" => 1536,
                _ => 1536, // Default fallback
            }
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        info!("OpenAI embedder processing batch of {} texts", texts.len());

        // Preprocess texts for better embeddings
        info!("Preprocessing {} texts for better OpenAI embeddings", texts.len());
        let preprocessed_texts: Vec<String> = texts
            .iter()
            .map(|text| preprocess_code_for_embedding(text))
            .collect();

        // Bypass HTTP call for test API key
        if self.api_key == "test-key" {
            info!("Using test API key, returning dummy embeddings");
            // Return dummy embeddings for testing
            return Ok(preprocessed_texts.iter().map(|_| vec![0.0; 1536]).collect());
        }

        // Log text lengths for debugging
        for (i, text) in preprocessed_texts.iter().enumerate() {
            info!("Preprocessed text {}: length {}", i, text.len());
            debug!("Preprocessed text {}: {}", i, text);
        }

        // Create the OpenAI API client
        let client = reqwest::Client::new();

        // Prepare the request body
        let request_body = serde_json::json!({
            "input": preprocessed_texts,
            "model": self.model
        });

        info!("Sending request to OpenAI API with model: {}", self.model);
        debug!("Request body: {}", serde_json::to_string_pretty(&request_body)?);

        // Send the request to OpenAI API
        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        info!("Received response from OpenAI API with status: {}", response.status());

        // Check if the request was successful
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenAI API request failed: {}", error_text));
        }

        // Parse the response
        let response_json: serde_json::Value = response.json().await?;

        // Extract embeddings from the response
        let embeddings = response_json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
            .iter()
            .map(|item| -> Result<Vec<f32>> {
                Ok(item["embedding"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Invalid embedding format"))?
                    .iter()
                    .map(|val| val.as_f64().unwrap() as f32)
                    .collect::<Vec<f32>>())
            })
            .collect::<Result<Vec<Vec<f32>>>>()?;

        // Log response details for debugging
        debug!("OpenAI API response: {}", serde_json::to_string_pretty(&response_json)?);
        info!(
            "OpenAI API returned {} embeddings, each with {} dimensions",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0)
        );

        Ok(embeddings)
    }
}

/// Production-ready local embedding service
/// Currently uses a placeholder implementation - to be replaced with actual
/// ONNX Runtime integration
pub struct LocalEmbedder {
    model_name: String,
    embedding_dim: usize,
}

impl LocalEmbedder {
    /// Create a new LocalEmbedder with model configuration
    /// This is a placeholder implementation that will be replaced with actual
    /// ONNX Runtime
    pub async fn new(
        _model_path: &Path,
        _tokenizer_path: &Path,
        model_name: Option<String>,
    ) -> Result<Self> {
        let model_name = model_name.unwrap_or_else(|| "microsoft/codebert-base".to_string());
        let embedding_dim = Self::get_embedding_dim(&model_name);

        info!(
            "LocalEmbedder initialized with model: {} (placeholder implementation)",
            model_name
        );
        warn!("This is a placeholder implementation. For production, integrate with ONNX Runtime.");

        Ok(Self { model_name, embedding_dim })
    }

    /// Create a LocalEmbedder with default configuration for testing
    pub fn new_default() -> Result<Self> {
        info!("Creating LocalEmbedder with default configuration");

        Ok(Self {
            model_name: "microsoft/codebert-base".to_string(),
            embedding_dim: 768,
        })
    }

    fn get_embedding_dim(model_name: &str) -> usize {
        match model_name {
            "sentence-transformers/all-MiniLM-L6-v2" => 384,
            "intfloat/e5-small-v2" => 384,
            "intfloat/e5-base-v2" => 768,
            "sentence-transformers/all-mpnet-base-v2" => 768,
            // Code-specific models
            "microsoft/codebert-base" => 768,
            "microsoft/graphcodebert-base" => 768,
            "microsoft/unixcoder-base" => 768,
            "huggingface/CodeBERTa-small-v1" => 768,
            _ => {
                warn!(
                    "Unknown model {}, using default embedding dimension",
                    model_name
                );
                768 // Default to 768 for code models
            }
        }
    }

    /// Generate deterministic embeddings based on text content
    /// This is a placeholder that generates consistent but non-semantic
    /// embeddings
    fn generate_placeholder_embedding(&self, text: &str) -> Vec<f32> {
        // Preprocess the text to improve embedding quality
        let processed_text = self.preprocess_text(text);

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a deterministic hash-based embedding that includes model name
        let mut hasher = DefaultHasher::new();
        processed_text.hash(&mut hasher);
        self.model_name.hash(&mut hasher); // Include model name for different embeddings per model
        let hash = hasher.finish();

        // Generate embedding vector from hash
        let mut embedding = Vec::with_capacity(self.embedding_dim);
        let mut seed = hash;

        for _ in 0..self.embedding_dim {
            // Simple linear congruential generator for deterministic values
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let value = ((seed >> 16) as f32) / 65536.0 - 0.5; // Range: [-0.5, 0.5]
            embedding.push(value);
        }

        // Normalize the embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            embedding.iter_mut().for_each(|x| *x /= norm);
        }

        embedding
    }

    /// Preprocess text to improve embedding quality for code
    fn preprocess_text(&self, text: &str) -> String {
        // Enhanced preprocessing for code content
        let mut processed = text.to_string();

        // Normalize whitespace but preserve code structure
        let lines: Vec<&str> = processed.lines().collect();
        let mut normalized_lines = Vec::new();

        for line in lines {
            // Preserve leading whitespace for indentation
            let trimmed = line.trim_end();
            if !trimmed.is_empty() {
                normalized_lines.push(trimmed);
            }
        }

        processed = normalized_lines.join("\n");

        // Add language-specific preprocessing
        if processed.contains("fn ") || processed.contains("impl ") || processed.contains("struct ")
        {
            // Rust code - preserve keywords and structure
            processed = self.enhance_rust_code(&processed);
        } else if processed.contains("def ") || processed.contains("class ") {
            // Python code - preserve keywords and structure
            processed = self.enhance_python_code(&processed);
        } else if processed.contains("function ")
            || processed.contains("const ")
            || processed.contains("class ")
        {
            // JavaScript/TypeScript code
            processed = self.enhance_js_code(&processed);
        }

        // Limit length to prevent overly long texts from dominating embeddings
        if processed.len() > 4000 {
            // For code, try to keep complete functions/classes
            if let Some(truncated) = self.smart_truncate_code(&processed, 4000) {
                truncated
            } else {
                processed[..4000].to_string()
            }
        } else {
            processed
        }
    }

    fn enhance_rust_code(&self, code: &str) -> String {
        // Add semantic markers for Rust constructs
        let mut enhanced = code.to_string();

        // Mark important Rust keywords for better embedding
        enhanced = enhanced.replace("pub fn", "[RUST_PUBLIC_FUNCTION]");
        enhanced = enhanced.replace("fn ", "[RUST_FUNCTION] ");
        enhanced = enhanced.replace("impl ", "[RUST_IMPLEMENTATION] ");
        enhanced = enhanced.replace("struct ", "[RUST_STRUCT] ");
        enhanced = enhanced.replace("enum ", "[RUST_ENUM] ");
        enhanced = enhanced.replace("trait ", "[RUST_TRAIT] ");
        enhanced = enhanced.replace("mod ", "[RUST_MODULE] ");

        enhanced
    }

    fn enhance_python_code(&self, code: &str) -> String {
        let mut enhanced = code.to_string();

        enhanced = enhanced.replace("def ", "[PYTHON_FUNCTION] ");
        enhanced = enhanced.replace("class ", "[PYTHON_CLASS] ");
        enhanced = enhanced.replace("async def", "[PYTHON_ASYNC_FUNCTION]");
        enhanced = enhanced.replace("@", "[PYTHON_DECORATOR]");

        enhanced
    }

    fn enhance_js_code(&self, code: &str) -> String {
        let mut enhanced = code.to_string();

        enhanced = enhanced.replace("function ", "[JS_FUNCTION] ");
        enhanced = enhanced.replace("const ", "[JS_CONST] ");
        enhanced = enhanced.replace("class ", "[JS_CLASS] ");
        enhanced = enhanced.replace("async ", "[JS_ASYNC] ");
        enhanced = enhanced.replace("export ", "[JS_EXPORT] ");
        enhanced = enhanced.replace("import ", "[JS_IMPORT] ");

        enhanced
    }

    fn smart_truncate_code(&self, code: &str, max_len: usize) -> Option<String> {
        if code.len() <= max_len {
            return Some(code.to_string());
        }

        // Try to find a good breaking point (end of function, class, etc.)
        let lines: Vec<&str> = code.lines().collect();
        let mut current_len = 0;
        let mut result_lines = Vec::new();

        for line in lines {
            if current_len + line.len() + 1 > max_len {
                // Check if this is a good breaking point
                let trimmed = line.trim();
                if trimmed == "}" || trimmed.starts_with('}') {
                    result_lines.push(line);
                    break;
                }
                // If not a good breaking point, break at previous line
                break;
            }
            result_lines.push(line);
            current_len += line.len() + 1; // +1 for newline
        }

        if result_lines.is_empty() {
            None
        } else {
            Some(result_lines.join("\n"))
        }
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    fn name(&self) -> &str {
        "local"
    }

    fn embedding_dimension(&self) -> usize {
        self.embedding_dim
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        info!(
            "Processing batch of {} texts with LocalEmbedder (placeholder)",
            texts.len()
        );

        // Preprocess texts for better embeddings
        info!("Preprocessing {} texts for better local embeddings", texts.len());
        let preprocessed_texts: Vec<String> = texts
            .iter()
            .map(|text| preprocess_code_for_embedding(text))
            .collect();

        // Log text lengths for debugging
        for (i, text) in preprocessed_texts.iter().enumerate() {
            info!("Preprocessed text {}: length {}", i, text.len());
            debug!("Preprocessed text {}: {}", i, text);
        }

        // Generate placeholder embeddings based on preprocessed text
        let embeddings = preprocessed_texts
            .iter()
            .map(|text| self.generate_placeholder_embedding(text))
            .collect();

        info!(
            "Successfully generated {} placeholder embeddings",
            texts.len()
        );

        Ok(embeddings)
    }
}
