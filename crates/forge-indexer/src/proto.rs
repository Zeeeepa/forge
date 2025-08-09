//! Protobuf definitions for gRPC communication

/// ChangeEvent represents a file change in a repository
#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub repo: String,
    pub branch: String,
    pub user_id: String,
    pub paths: Vec<String>,
    pub sha: String,
}

/// Chunk represents a code chunk with its metadata
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub path: String,
    pub lang: String,
    pub symbol: Option<String>,
    pub rev: String,
    pub size: usize,
    pub code: String,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

/// RetrievalRequest represents a request to retrieve relevant code chunks
#[derive(Debug, Clone)]
pub struct RetrievalRequest {
    pub query: String,
    pub repo: String,
    pub branch: String,
    pub user_id: String,
    pub file_hashes: std::collections::HashMap<String, String>, // path -> sha256
    pub k: usize,
}

/// RetrievalResponse represents the response to a retrieval request
#[derive(Debug, Clone)]
pub struct RetrievalResponse {
    pub chunks: Vec<RetrievedChunk>,
}

/// RetrievedChunk represents a chunk returned in a retrieval response
#[derive(Debug, Clone, serde::Serialize)]
pub struct RetrievedChunk {
    pub code: String,
    pub path: String,
    pub score: f32,
    pub chunk_hash: String,
}
