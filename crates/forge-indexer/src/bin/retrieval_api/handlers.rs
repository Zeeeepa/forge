use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use forge_indexer::proto::RetrievalRequest;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::service::handle_retrieval_request;
use super::state::AppState;
use super::types::{
    ErrorResponse, HealthQuery, HealthResponse, HttpRetrievalRequest, HttpRetrievalResponse,
};
use super::validation::validate_proof_of_possession;

/// Health check endpoint
pub async fn health_handler(
    Query(params): Query<HealthQuery>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = Uuid::new_v4().to_string();

    info!(request_id = %request_id, "Health check requested");

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Track actual uptime
    };

    if params.detailed {
        info!(request_id = %request_id, "Detailed health check completed");
    }

    Ok(Json(response))
}

/// Main retrieval endpoint with proof-of-possession validation
pub async fn retrieve_handler(
    State(state): State<AppState>,
    Json(req): Json<HttpRetrievalRequest>,
) -> Result<Json<HttpRetrievalResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();

    info!(
        request_id = %request_id,
        query = %req.query,
        repo = %req.repo,
        branch = %req.branch,
        user_id = %req.user_id,
        k = req.k,
        file_count = req.file_hashes.len(),
        "Retrieval request received"
    );

    // Validate proof-of-possession
    match validate_proof_of_possession(&req.file_hashes).await {
        Ok(()) => {
            info!(request_id = %request_id, "Proof-of-possession validation successful");
        }
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                "Proof-of-possession validation failed"
            );
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Proof-of-possession validation failed".to_string(),
                    request_id: request_id.clone(),
                    code: "INVALID_PROOF".to_string(),
                }),
            ));
        }
    }

    // Convert to internal request format
    let internal_req = RetrievalRequest {
        query: req.query.clone(),
        repo: req.repo.clone(),
        branch: req.branch.clone(),
        user_id: req.user_id.clone(),
        file_hashes: req.file_hashes.clone(),
        k: req.k,
    };

    // Process retrieval request
    match handle_retrieval_request(&state, internal_req).await {
        Ok(response) => {
            let processing_time = start_time.elapsed();
            info!(
                request_id = %request_id,
                chunks_found = response.chunks.len(),
                processing_time_ms = processing_time.as_millis(),
                "Retrieval request completed successfully"
            );

            Ok(Json(HttpRetrievalResponse {
                request_id,
                total_found: response.chunks.len(),
                chunks: response.chunks,
                processing_time_ms: processing_time.as_millis() as u64,
                stats: std::collections::HashMap::new(),
            }))
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "Retrieval request failed"
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Retrieval failed: {e}"),
                    request_id,
                    code: "RETRIEVAL_ERROR".to_string(),
                }),
            ))
        }
    }
}
