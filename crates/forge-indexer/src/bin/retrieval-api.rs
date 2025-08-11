//! Retrieval API service with HTTP/gRPC endpoints and proof-of-possession
//! validation

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::anyhow;
use axum::Router;
use axum::routing::{get, post};
use forge_indexer::{Embedder, IndexService, init_production_logging};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

mod retrieval_api;

use retrieval_api::{AppState, health_handler, retrieve_handler};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize production logging
    init_production_logging().map_err(|e| anyhow!("Failed to initialize logging: {}", e))?;

    info!("Starting Forge Retrieval API server");

    // Create embedder (using local embedder for development)
    let embedder = Arc::new(
        forge_indexer::embedder::LocalEmbedder::new_default()
            .map_err(|e| anyhow!("Failed to create local embedder: {}", e))?,
    );

    // Initialize components
    let vector_dimension = embedder.embedding_dimension();
    let index_service = Arc::new(
        IndexService::new(vector_dimension)
            .await
            .map_err(|e| anyhow!("Failed to create index service: {}", e))?,
    );

    let app_state = AppState { index_service, embedder };

    // Build router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/retrieve", post(retrieve_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(app_state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3014));
    info!("Forge Retrieval API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
