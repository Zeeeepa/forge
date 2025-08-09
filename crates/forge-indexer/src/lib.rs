//! Forge Indexer - Real-time codebase indexing service

pub mod chunker;
pub mod embedder;
pub mod errors;
pub mod index_svc;
pub mod logging;
pub mod pipeline;
pub mod proto;

pub mod watcher;

pub use chunker::Chunker;
pub use embedder::Embedder;
pub use errors::{ForgeIndexerError, Result};
pub use index_svc::IndexService;
pub use logging::{
    LoggingConfig, init_default_logging, init_development_logging, init_production_logging,
};
pub use pipeline::{EmbedderType, IndexingPipeline, PipelineConfig};
pub use watcher::FileWatcher;
