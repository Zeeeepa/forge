use std::collections::HashMap;

use forge_indexer::proto::Chunk;
use serde::{Deserialize, Serialize};

/// HTTP request for retrieval endpoint
#[derive(Debug, Deserialize)]
pub struct HttpRetrievalRequest {
    pub query: String,
    pub repo: String,
    pub branch: String,
    pub user_id: String,
    pub file_hashes: HashMap<String, String>, // path -> sha256
    #[serde(default = "default_k")]
    pub k: usize,
}

fn default_k() -> usize {
    10
}

/// HTTP response for retrieval endpoint
#[derive(Debug, Serialize)]
pub struct HttpRetrievalResponse {
    pub request_id: String,
    pub chunks: Vec<Chunk>,
    pub total_found: usize,
    pub processing_time_ms: u64,
    pub stats: HashMap<String, String>,
}

/// Error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub request_id: String,
    pub code: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Query parameters for health endpoint
#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    #[serde(default)]
    pub detailed: bool,
}
