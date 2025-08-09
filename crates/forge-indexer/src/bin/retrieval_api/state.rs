use std::sync::Arc;

use forge_indexer::{Embedder, IndexService};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub index_service: Arc<IndexService>,
    pub embedder: Arc<dyn Embedder>,
}
