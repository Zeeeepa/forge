use forge_indexer::proto::{RetrievalRequest, RetrievalResponse, RetrievedChunk};
use forge_indexer::{ForgeIndexerError, Result as ForgeResult};
use tracing::{error, info};

use super::state::AppState;

/// Handle retrieval request with simple vector search
pub async fn handle_retrieval_request(
    state: &AppState,
    request: RetrievalRequest,
) -> ForgeResult<RetrievalResponse> {
    info!(
        "Starting retrieval request for query: '{}', repo: '{}', branch: '{}'",
        request.query, request.repo, request.branch
    );

    // Generate embedding for the query
    let query_embedding = state.embedder.embed(&request.query).await.map_err(|e| {
        error!("Failed to generate query embedding: {}", e);
        ForgeIndexerError::embedding_error(format!("Failed to generate query embedding: {e}"))
    })?;

    info!(
        "Generated query embedding with {} dimensions",
        query_embedding.len()
    );

    // Search the vector database
    let search_results = state
        .index_service
        .search_similar(&query_embedding, request.k, None, None)
        .await
        .map_err(|e| {
            error!("Vector search failed: {}", e);
            ForgeIndexerError::vector_db_error(format!("Vector search failed: {e}"))
        })?;

    info!(
        "Found {} search results from vector database",
        search_results.len()
    );

    // Convert to response format
    let final_results: Vec<RetrievedChunk> = search_results
        .into_iter()
        .map(|(chunk, score)| {
            info!("Result: path={}, score={:.4}", chunk.path, score);
            RetrievedChunk {
                code: chunk.code,
                path: chunk.path,
                score,
                chunk_hash: chunk.id,
            }
        })
        .collect();

    info!("Returning {} chunks to client", final_results.len());

    Ok(RetrievalResponse { chunks: final_results })
}
