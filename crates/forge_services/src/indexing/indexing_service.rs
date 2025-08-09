//! Indexing service implementation following established patterns

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use forge_domain::{
    ChunkingConfig, CodeChunk, Embedder, Chunker, EmbeddingProvider, IndexedCodebase, 
    IndexingConfig, IndexingProgress, IndexingRequest, IndexingResponse, IndexingStage, 
    IndexingStatistics, IndexingStatus, ProcessingTimeBreakdown,
};
use forge_walker::Walker;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

use crate::vector_store::{SharedVectorStore, VectorStoreFactory};

/// Service for indexing codebases with configurable chunking and embedding
/// strategies
pub struct IndexingService {
    vector_store: SharedVectorStore,
    chunker: Arc<RwLock<Box<dyn Chunker>>>,
    embedder: Arc<RwLock<Box<dyn Embedder>>>,
    config: IndexingConfig,
    progress_sender: Option<mpsc::UnboundedSender<IndexingProgress>>,
}

impl IndexingService {
    /// Create a new indexing service with the given configuration
    pub async fn new(
        config: IndexingConfig,
        chunker: Box<dyn Chunker>,
        embedder: Box<dyn Embedder>,
    ) -> Result<Self> {
        info!("Initializing IndexingService");

        // Create vector store
        let vector_store = VectorStoreFactory::create(&config.vector_store)
            .await
            .context("Failed to create vector store")?;
        let shared_store = Arc::new(RwLock::new(vector_store));

        // Ensure collection exists
        {
            let mut store = shared_store.write().await;
            let collection_name = &config.vector_store.collection_name;
            let dimension = embedder.embedding_dimension();

            if !store.collection_exists(collection_name).await? {
                info!(
                    "Creating collection '{}' with dimension {}",
                    collection_name, dimension
                );
                store.create_collection(collection_name, dimension).await?;
            }
        }

        Ok(Self {
            vector_store: shared_store,
            chunker: Arc::new(RwLock::new(chunker)),
            embedder: Arc::new(RwLock::new(embedder)),
            config,
            progress_sender: None,
        })
    }

    /// Set progress callback for indexing operations
    pub fn set_progress_callback(&mut self, sender: mpsc::UnboundedSender<IndexingProgress>) {
        self.progress_sender = Some(sender);
    }

    /// Get the current indexing configuration
    pub fn config(&self) -> &IndexingConfig {
        &self.config
    }

    /// Index a codebase according to the request
    pub async fn index_codebase(&self, request: IndexingRequest) -> Result<IndexingResponse> {
        let start_time = Instant::now();
        let mut stats = IndexingStatistics {
            files_discovered: 0,
            files_processed: 0,
            files_skipped: 0,
            files_failed: 0,
            chunks_created: 0,
            embeddings_generated: 0,
            bytes_processed: 0,
            time_breakdown: ProcessingTimeBreakdown {
                file_discovery_ms: 0,
                file_reading_ms: 0,
                chunking_ms: 0,
                embedding_ms: 0,
                storage_ms: 0,
                total_ms: 0,
            },
            error_summary: HashMap::new(),
            language_distribution: HashMap::new(),
        };

        let warnings = Vec::new();

        // Reset existing index if requested
        if request.reset_existing {
            info!(
                "Resetting existing index for collection: {}",
                request.config.vector_store.collection_name
            );
            let mut store = self.vector_store.write().await;
            store
                .delete_collection(&request.config.vector_store.collection_name)
                .await?;
            store
                .create_collection(
                    &request.config.vector_store.collection_name,
                    self.embedder.read().await.embedding_dimension(),
                )
                .await?;
        }

        // Stage 1: File Discovery
        self.send_progress(
            &request.request_id,
            IndexingStage::Discovery,
            0.0,
            None,
            0,
            0,
            0,
        )
        .await;
        let discovery_start = Instant::now();

        let files_to_process = if !request.specific_files.is_empty() {
            request.specific_files.clone()
        } else {
            self.discover_files(&request.root_path, &request.config.filtering)
                .await?
        };

        stats.files_discovered = files_to_process.len();
        stats.time_breakdown.file_discovery_ms = discovery_start.elapsed().as_millis() as u64;

        info!("Discovered {} files for indexing", stats.files_discovered);

        // Stage 2: Process files (Chunking, Embedding, Storage)
        self.send_progress(
            &request.request_id,
            IndexingStage::Chunking,
            0.1,
            None,
            0,
            stats.files_discovered,
            0,
        )
        .await;

        for (file_index, file_path) in files_to_process.iter().enumerate() {
            let file_start = Instant::now();

            match self
                .process_single_file(file_path, &request, &mut stats)
                .await
            {
                Ok(()) => {
                    stats.files_processed += 1;
                }
                Err(e) => {
                    stats.files_failed += 1;
                    let error_key = format!("{e}");
                    *stats.error_summary.entry(error_key).or_insert(0) += 1;
                    warn!("Failed to process file {:?}: {}", file_path, e);
                }
            }

            // Update progress
            let progress = (file_index + 1) as f32 / stats.files_discovered as f32;
            let stage = if progress < 0.6 {
                IndexingStage::Chunking
            } else if progress < 0.9 {
                IndexingStage::Embedding
            } else {
                IndexingStage::Storage
            };

            self.send_progress(
                &request.request_id,
                stage,
                0.1 + progress * 0.8,
                Some(file_path.to_string_lossy().to_string()),
                stats.files_processed,
                stats.files_discovered,
                stats.chunks_created,
            )
            .await;

            // Add processing time
            let file_time = file_start.elapsed().as_millis() as u64;
            stats.time_breakdown.file_reading_ms += file_time;
        }

        // Stage 3: Finalization
        self.send_progress(
            &request.request_id,
            IndexingStage::Finalization,
            0.95,
            None,
            stats.files_processed,
            stats.files_discovered,
            stats.chunks_created,
        )
        .await;

        stats.time_breakdown.total_ms = start_time.elapsed().as_millis() as u64;

        // Create indexed codebase record
        let codebase = IndexedCodebase {
            id: request.request_id.clone(),
            repository: request.repository.clone(),
            branch: request.branch.clone(),
            files_count: stats.files_processed,
            chunks_count: stats.chunks_created,
            indexed_at: chrono::Utc::now(),
            config: request.config.chunking.clone(),
            language_stats: stats.language_distribution.clone(),
            total_size_bytes: stats.bytes_processed,
            status: if stats.files_failed == 0 {
                IndexingStatus::Completed
            } else {
                IndexingStatus::Failed(format!("{} files failed to process", stats.files_failed))
            },
        };

        // Send completion
        self.send_progress(
            &request.request_id,
            IndexingStage::Completed,
            1.0,
            None,
            stats.files_processed,
            stats.files_discovered,
            stats.chunks_created,
        )
        .await;

        info!(
            "Indexing completed: {} files processed, {} chunks created, {} embeddings generated",
            stats.files_processed, stats.chunks_created, stats.embeddings_generated
        );

        Ok(IndexingResponse {
            request_id: request.request_id,
            codebase,
            statistics: stats,
            warnings,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Process a single file through the indexing pipeline
    async fn process_single_file(
        &self,
        file_path: &Path,
        request: &IndexingRequest,
        stats: &mut IndexingStatistics,
    ) -> Result<()> {
        debug!("Processing file: {:?}", file_path);

        // Read file content
        let content = tokio::fs::read_to_string(file_path)
            .await
            .context("Failed to read file")?;

        stats.bytes_processed += content.len() as u64;

        // Detect language
        let chunker = self.chunker.read().await;
        let language = chunker
            .detect_language(file_path)
            .unwrap_or_else(|| "text".to_string());

        // Update language distribution
        *stats
            .language_distribution
            .entry(language.clone())
            .or_insert(0) += 1;

        // Chunk the file
        let chunking_start = Instant::now();
        let chunks = chunker
            .chunk_file(
                &file_path.to_string_lossy(),
                &content,
                &language,
                &request.branch, // Using branch as revision for now
                &request.config.chunking,
            )
            .await
            .context("Failed to chunk file")?;

        stats.time_breakdown.chunking_ms += chunking_start.elapsed().as_millis() as u64;
        stats.chunks_created += chunks.len();

        if chunks.is_empty() {
            debug!("No chunks created for file: {:?}", file_path);
            return Ok(());
        }

        // Generate embeddings
        let embedding_start = Instant::now();
        let embedder = self.embedder.read().await;
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = embedder
            .embed_batch(&texts)
            .await
            .context("Failed to generate embeddings")?;

        stats.time_breakdown.embedding_ms += embedding_start.elapsed().as_millis() as u64;
        stats.embeddings_generated += embeddings.len();

        // Store in vector database
        let storage_start = Instant::now();
        let chunk_embedding_pairs: Vec<(CodeChunk, Vec<f32>)> =
            chunks.into_iter().zip(embeddings.into_iter()).collect();

        let mut store = self.vector_store.write().await;
        store
            .insert_chunks(
                &request.config.vector_store.collection_name,
                &chunk_embedding_pairs,
            )
            .await
            .context("Failed to store chunks in vector database")?;

        stats.time_breakdown.storage_ms += storage_start.elapsed().as_millis() as u64;

        debug!(
            "Successfully processed file: {:?} ({} chunks)",
            file_path,
            chunk_embedding_pairs.len()
        );
        Ok(())
    }

    /// Discover files to index based on configuration
    async fn discover_files(
        &self,
        root_path: &Path,
        filter_config: &forge_domain::FilterConfig,
    ) -> Result<Vec<PathBuf>> {
        info!("Discovering files in: {:?}", root_path);

        let walker = Walker::min_all().cwd(root_path.to_path_buf());
        let files = walker.get().await?;

        let filtered_files: Vec<PathBuf> = files
            .into_iter()
            .filter(|file| !file.is_dir())
            .map(|file| root_path.join(&file.path))
            .filter(|path| self.should_include_file(path, filter_config))
            .collect();

        info!("Filtered to {} files for indexing", filtered_files.len());
        Ok(filtered_files)
    }

    /// Check if a file should be included based on filter configuration
    fn should_include_file(&self, path: &Path, config: &forge_domain::FilterConfig) -> bool {
        // Check file size
        if let Ok(metadata) = std::fs::metadata(path) {
            let size = metadata.len();
            if size < config.min_file_size_bytes || size > config.max_file_size_bytes {
                return false;
            }
        }

        // Check extension
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str())
            && !config
                .supported_extensions
                .contains(&extension.to_lowercase())
            {
                return false;
            }

        // Check ignore patterns
        let path_str = path.to_string_lossy();
        for pattern in &config.ignore_patterns {
            if glob_match(pattern, &path_str) {
                return false;
            }
        }

        // Check ignore directories
        for ignore_dir in &config.ignore_directories {
            if path_str.contains(&format!("/{ignore_dir}/"))
                || path_str.contains(&format!("\\{ignore_dir}\\"))
            {
                return false;
            }
        }

        true
    }

    /// Send progress update if callback is configured
    async fn send_progress(
        &self,
        request_id: &str,
        stage: IndexingStage,
        progress_percent: f32,
        current_file: Option<String>,
        files_processed: usize,
        total_files: usize,
        chunks_created: usize,
    ) {
        if let Some(sender) = &self.progress_sender {
            let progress = IndexingProgress {
                request_id: request_id.to_string(),
                stage,
                progress_percent: (progress_percent * 100.0).min(100.0),
                current_file,
                files_processed,
                total_files,
                chunks_created,
                estimated_remaining_seconds: None, // Could be calculated based on throughput
                throughput_fps: 0.0,               // Could be calculated
            };

            if let Err(e) = sender.send(progress) {
                warn!("Failed to send progress update: {}", e);
            }
        }
    }
}

/// Simple glob pattern matching (basic implementation)
fn glob_match(pattern: &str, text: &str) -> bool {
    // Very basic glob matching - just check for wildcards
    if pattern.contains("**") {
        // Recursive wildcard - check if any part matches
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    } else if pattern.contains('*') {
        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                if !text[pos..].starts_with(part) {
                    return false;
                }
                pos += part.len();
            } else if i == parts.len() - 1 {
                return text[pos..].ends_with(part);
            } else if let Some(found) = text[pos..].find(part) {
                pos += found + part.len();
            } else {
                return false;
            }
        }
        return true;
    }

    // Exact match
    text.contains(pattern)
}
