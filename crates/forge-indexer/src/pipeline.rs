//! Production-grade indexing pipeline

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use forge_walker::Walker;
use ignore::gitignore::Gitignore;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

use crate::chunker::{self, Chunker};
use crate::embedder::{Embedder, LocalEmbedder, OpenAIEmbedder};
use crate::index_svc::IndexService;
use crate::proto::Chunk;
use crate::watcher::FileWatcher;

/// Configuration for the indexing pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub embedder_type: EmbedderType,
    pub openai_api_key: Option<String>,
    pub local_model_path: Option<PathBuf>,
    pub local_tokenizer_path: Option<PathBuf>,
    pub batch_size: usize,
    pub max_concurrent_files: usize,
    pub supported_extensions: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum EmbedderType {
    OpenAI,
    Local,
    Hybrid, // Use local with OpenAI fallback
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            embedder_type: EmbedderType::OpenAI,
            openai_api_key: None,
            local_model_path: None,
            local_tokenizer_path: None,
            batch_size: 10,
            max_concurrent_files: 5,
            supported_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "jsx".to_string(),
                "tsx".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "cc".to_string(),
                "cxx".to_string(),
                "c".to_string(),
                "h".to_string(),
                "hpp".to_string(),
                "go".to_string(),
                "rb".to_string(),
                "scala".to_string(),
                "cs".to_string(),
                "php".to_string(),
                "swift".to_string(),
                "kt".to_string(),
                "kts".to_string(),
                "css".to_string(),
                "scss".to_string(),
                "sass".to_string(),
                "less".to_string(),
                "html".to_string(),
                "htm".to_string(),
                "xml".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "toml".to_string(),
                "md".to_string(),
                "markdown".to_string(),
                "rst".to_string(),
                "txt".to_string(),
            ],
        }
    }
}

/// Statistics for the indexing pipeline
#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub files_processed: u64,
    pub chunks_created: u64,
    pub embeddings_generated: u64,
    pub errors_encountered: u64,
    pub bytes_processed: u64,
}

/// Production-grade indexing pipeline
pub struct IndexingPipeline {
    config: PipelineConfig,
    chunker: Arc<RwLock<Chunker>>,
    embedder: Arc<dyn Embedder>,
    index_service: Arc<RwLock<IndexService>>,
    stats: Arc<RwLock<PipelineStats>>,
    file_watcher: Option<FileWatcher>,

    gitignore: Option<Gitignore>,
    event_receiver: Option<mpsc::Receiver<notify::Event>>,
    walker: Option<Walker>,
}

impl IndexingPipeline {
    /// Create a new indexing pipeline with the given configuration

    pub async fn new_from_config(
        embedder_type: String,
        openai_api_key: Option<String>,
        local_model_path: Option<String>,
        local_tokenizer_path: Option<String>,
        batch_size: usize,
        max_concurrent_files: usize,
        supported_extensions: Vec<String>,
    ) -> Result<Self> {
        let embedder_type = match embedder_type.as_str() {
            "openai" => EmbedderType::OpenAI,
            "local" => EmbedderType::Local,
            "hybrid" => EmbedderType::Hybrid,
            _ => return Err(anyhow::anyhow!("Invalid embedder type: {}", embedder_type)),
        };

        let local_model_path = local_model_path.map(PathBuf::from);
        let local_tokenizer_path = local_tokenizer_path.map(PathBuf::from);

        let config = PipelineConfig {
            embedder_type,
            openai_api_key,
            local_model_path,
            local_tokenizer_path,
            batch_size,
            max_concurrent_files,
            supported_extensions,
        };

        Self::new(config).await
    }

    /// Check if a file should be ignored based on various criteria
    pub fn should_ignore_file(&self, file_path: &Path) -> bool {
        // Explicitly ignore target directory and its contents
        if let Some(path_str) = file_path.to_str()
            && (path_str.contains("/target/")
                || path_str.contains("\\target\\")
                || path_str.starts_with("target/")
                || path_str.starts_with("target\\")
                || path_str == "target")
        {
            debug!("🚫 Ignoring target directory file: {:?}", file_path);
            return true;
        }

        // Also ignore common binary files and system files
        if let Some(extension) = file_path.extension().and_then(|ext| ext.to_str()) {
            let binary_extensions = [
                "exe", "dll", "so", "dylib", "a", "lib", "obj", "o", "bin", "class", "jar", "war",
                "ear", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "pdf", "doc", "docx", "xls",
                "xlsx", "ppt", "pptx", "jpg", "jpeg", "png", "gif", "bmp", "svg", "ico", "mp3",
                "mp4", "avi", "mov", "wmv", "flv", "db", "sqlite", "sqlite3", "db-shm", "db-wal",
            ];

            if binary_extensions.contains(&extension.to_lowercase().as_str()) {
                debug!("🚫 Ignoring binary file: {:?}", file_path);
                return true;
            }
        }

        // Ignore system files
        if let Some(filename) = file_path.file_name().and_then(|name| name.to_str()) {
            let system_files = [".DS_Store", "Thumbs.db", "desktop.ini"];
            if system_files.contains(&filename) {
                debug!("🚫 Ignoring system file: {:?}", file_path);
                return true;
            }
        }

        // Ignore vendor directories and other common build artifacts
        if let Some(path_str) = file_path.to_str() {
            let ignore_patterns = [
                "/vendor/",
                "\\vendor\\",
                "/.git/",
                "\\.git\\",
                "/node_modules/",
                "\\node_modules\\",
                "/.fastembed_cache/",
                "\\.fastembed_cache\\",
                "/debug/",
                "\\debug\\",
            ];

            for pattern in &ignore_patterns {
                if path_str.contains(pattern) {
                    debug!("🚫 Ignoring directory pattern: {:?}", file_path);
                    return true;
                }
            }
        }

        false
    }

    /// Filter a list of files based on gitignore patterns and supported
    /// extensions
    pub fn filter_files(&self, file_paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let start_time = std::time::Instant::now();
        let initial_count = file_paths.len();

        let filtered_files: Vec<PathBuf> = file_paths
            .into_iter()
            .filter(|path| {
                // Skip if should be ignored
                if self.should_ignore_file(path) {
                    return false;
                }

                // Check if extension is supported
                if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                    let ext_lower = extension.to_lowercase();
                    if self.config.supported_extensions.contains(&ext_lower) {
                        return true;
                    }
                }

                // Allow files without extensions (like README, LICENSE, etc.)
                if path.extension().is_none()
                    && let Some(filename) = path.file_name().and_then(|name| name.to_str())
                {
                    let common_text_files = [
                        "README",
                        "LICENSE",
                        "CHANGELOG",
                        "CONTRIBUTING",
                        "AUTHORS",
                        "INSTALL",
                        "NEWS",
                        "TODO",
                        "COPYING",
                    ];
                    return common_text_files
                        .iter()
                        .any(|&name| filename.to_uppercase().starts_with(name));
                }

                false
            })
            .collect();

        let filtered_count = filtered_files.len();
        let filter_duration = start_time.elapsed();

        info!(
            "🔍 File filtering complete: {} -> {} files ({} filtered out) in {:?}",
            initial_count,
            filtered_count,
            initial_count - filtered_count,
            filter_duration
        );

        filtered_files
    }
    /// Recursively collect all files in a directory, respecting gitignore
    /// patterns
    pub async fn collect_files_from_directory(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let start_time = std::time::Instant::now();
        info!("📂 Collecting files from directory: {:?}", dir_path);

        // Use forge_walker to collect files, respecting gitignore patterns
        let walker = self
            .walker
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Walker not initialized"))?;
        let walker = walker.clone().cwd(dir_path.to_path_buf());

        let walker_files = walker.get().await?;

        let collection_duration = start_time.elapsed();
        info!(
            "📁 File collection complete: {} files found in {:?}",
            walker_files.len(),
            collection_duration
        );

        // Convert Walker::File to PathBuf and filter files
        let all_files: Vec<PathBuf> = walker_files
            .into_iter()
            .filter(|file| !file.is_dir()) // Only process files, not directories
            .map(|file| dir_path.join(&file.path))
            .collect();

        // Filter the collected files using existing logic
        let filtered_files = self.filter_files(all_files);

        Ok(filtered_files)
    }
    /// Load gitignore patterns from the repository (deprecated - using
    /// forge_walker now)
    async fn load_gitignore_patterns() -> Option<Gitignore> {
        // Try to load .gitignore from the current directory
        let gitignore_path = std::env::current_dir().ok()?.join(".gitignore");
        if !gitignore_path.exists() {
            return None;
        }

        let (gitignore, error) = Gitignore::new(&gitignore_path);
        if let Some(e) = error {
            warn!("⚠️  Failed to load gitignore patterns: {}", e);
            None
        } else {
            info!("📄 Loaded gitignore patterns from {:?}", gitignore_path);
            Some(gitignore)
        }
    }

    pub async fn new(config: PipelineConfig) -> Result<Self> {
        info!(
            "🔧 Initializing IndexingPipeline with embedder type: {:?}",
            config.embedder_type
        );
        info!(
            "📊 Pipeline configuration - Batch size: {}, Max concurrent files: {}, Supported extensions: {}",
            config.batch_size,
            config.max_concurrent_files,
            config.supported_extensions.len()
        );

        // Initialize chunker
        info!("📝 Initializing chunker...");
        let chunker = Arc::new(RwLock::new(Chunker::new()));

        // Initialize embedder based on configuration
        info!("🤖 Initializing embedder: {:?}", config.embedder_type);
        let embedder: Arc<dyn Embedder> = match config.embedder_type {
            EmbedderType::OpenAI => {
                let _api_key = config.openai_api_key.clone().ok_or_else(|| {
                    anyhow::anyhow!("OpenAI API key required for OpenAI embedder")
                })?;
                info!("🔗 Using OpenAI embedder");
                Arc::new(OpenAIEmbedder::new().await?)
            }
            EmbedderType::Local => {
                if let (Some(model_path), Some(tokenizer_path)) =
                    (&config.local_model_path, &config.local_tokenizer_path)
                {
                    info!("📁 Using local embedder with custom model paths");
                    info!("   Model: {:?}", model_path);
                    info!("   Tokenizer: {:?}", tokenizer_path);
                    Arc::new(LocalEmbedder::new(model_path, tokenizer_path, None).await?)
                } else {
                    warn!("⚠️  No model paths provided for local embedder, using default");
                    Arc::new(LocalEmbedder::new_default()?)
                }
            }
            EmbedderType::Hybrid => {
                // For now, just use local embedder
                // In production, this would implement fallback logic
                warn!("⚠️  Hybrid embedder not fully implemented, using local embedder");
                Arc::new(LocalEmbedder::new_default()?)
            }
        };

        // Initialize index service
        info!("🗂️  Initializing index service...");
        let vector_dimension = embedder.embedding_dimension();
        let index_service = Arc::new(RwLock::new(IndexService::new(vector_dimension).await?));

        // Load gitignore patterns if available
        let gitignore = Self::load_gitignore_patterns().await;

        // Create a walker instance
        let walker = Some(
            Walker::min_all().cwd(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        );

        info!("✅ IndexingPipeline initialization complete");

        Ok(Self {
            config,
            chunker,
            embedder,
            index_service,
            stats: Arc::new(RwLock::new(PipelineStats::default())),
            file_watcher: None,
            event_receiver: None,
            gitignore,
            walker,
        })
    }

    /// Start watching a directory for file changes
    pub async fn start_watching(&mut self, watch_path: &Path) -> Result<()> {
        info!("👀 Starting file watcher for path: {:?}", watch_path);

        if !watch_path.exists() {
            error!("❌ Watch path does not exist: {:?}", watch_path);
            return Err(anyhow::anyhow!("Watch path does not exist"));
        }

        let (mut watcher, receiver) = FileWatcher::new()?;
        watcher.watch_directory(watch_path)?;

        self.file_watcher = Some(watcher);
        self.event_receiver = Some(receiver);

        info!("✅ File watcher started successfully for {:?}", watch_path);
        Ok(())
    }

    /// Process file change events from the watcher
    pub async fn process_events(&mut self) -> Result<()> {
        let mut receiver = self
            .event_receiver
            .take()
            .ok_or_else(|| anyhow::anyhow!("File watcher not started"))?;

        info!("🔄 Starting event processing loop - waiting for file changes...");
        let mut event_count = 0;

        while let Some(event) = receiver.recv().await {
            event_count += 1;
            debug!("📨 Received file event #{}: {:?}", event_count, event);

            let start_time = std::time::Instant::now();
            if let Err(e) = self.handle_file_event(event).await {
                error!("❌ Error handling file event #{}: {}", event_count, e);
                let mut stats = self.stats.write().await;
                stats.errors_encountered += 1;
            } else {
                let duration = start_time.elapsed();
                debug!("✅ File event #{} processed in {:?}", event_count, duration);
            }

            // Log periodic statistics
            if event_count % 10 == 0 {
                let stats = self.stats.read().await;
                info!(
                    "📊 Progress update - Events: {}, Files: {}, Chunks: {}, Embeddings: {}, Errors: {}",
                    event_count,
                    stats.files_processed,
                    stats.chunks_created,
                    stats.embeddings_generated,
                    stats.errors_encountered
                );
            }
        }

        info!(
            "🏁 Event processing loop completed after {} events",
            event_count
        );
        Ok(())
    }

    /// Handle a single file change event
    async fn handle_file_event(&self, event: notify::Event) -> Result<()> {
        debug!("🔍 Analyzing file event: {:?}", event);

        let mut processed_files = 0;
        for path in event.paths {
            if self.should_process_file(&path) {
                debug!("✅ File eligible for processing: {:?}", path);
                let start_time = std::time::Instant::now();

                match self.process_file(&path).await {
                    Ok(()) => {
                        processed_files += 1;
                        let duration = start_time.elapsed();
                        info!(
                            "✅ Successfully processed file: {:?} (took {:?})",
                            path, duration
                        );
                    }
                    Err(e) => {
                        error!("❌ Error processing file {:?}: {}", path, e);
                        let mut stats = self.stats.write().await;
                        stats.errors_encountered += 1;
                    }
                }
            } else {
                debug!("⏭️  Skipping file (not eligible): {:?}", path);
            }
        }

        if processed_files > 0 {
            debug!(
                "📈 Event processing complete - {} files processed",
                processed_files
            );
        }

        Ok(())
    }

    /// Check if a file should be processed based on extension and other
    /// criteria
    fn should_process_file(&self, path: &Path) -> bool {
        // Check if file exists and is a file
        if !path.is_file() {
            return false;
        }

        // Check if file should be ignored (gitignore, binary files, etc.)
        if self.should_ignore_file(path) {
            return false;
        }

        // Check extension
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let ext_lower = extension.to_lowercase();
            if self.config.supported_extensions.contains(&ext_lower) {
                return true;
            }
        }

        // Allow files without extensions (like README, LICENSE, etc.)
        if path.extension().is_none()
            && let Some(filename) = path.file_name().and_then(|name| name.to_str())
        {
            let common_text_files = [
                "README",
                "LICENSE",
                "CHANGELOG",
                "CONTRIBUTING",
                "AUTHORS",
                "INSTALL",
                "NEWS",
                "TODO",
                "COPYING",
            ];
            return common_text_files
                .iter()
                .any(|&name| filename.to_uppercase().starts_with(name));
        }

        false
    }

    /// Process a single file: read, chunk, embed, and index
    pub async fn process_file(&self, file_path: &Path) -> Result<()> {
        let start_time = std::time::Instant::now();
        info!("📄 Processing file: {:?}", file_path);

        // Read file content
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                info!(
                    "📖 Successfully read file content ({} bytes) from {:?}",
                    content.len(),
                    file_path
                );
                content
            }
            Err(e) => {
                error!("❌ Failed to read file {:?}: {}", file_path, e);
                return Err(anyhow::anyhow!("Failed to read file: {}", e));
            }
        };

        let file_size = content.len() as u64;

        // Determine language from extension
        let language = self.get_language_from_path(file_path);
        info!(
            "🏷️  Detected language: {} for file {:?}",
            language, file_path
        );

        // Generate a revision hash for the file
        let revision = self.generate_file_hash(&content);
        info!(
            "🔑 Generated file hash: {} for {:?}",
            &revision[..8],
            file_path
        );

        // Chunk the file
        let chunks = {
            let chunk_start = std::time::Instant::now();
            info!("🧩 Starting chunking process for {:?}", file_path);
            let mut chunker = self.chunker.write().await;
            let result = chunker.chunk_file(
                file_path.to_string_lossy().as_ref(),
                &content,
                &language,
                &revision,
            );
            let chunk_duration = chunk_start.elapsed();

            match result {
                Ok(chunks) => {
                    info!(
                        "🧩 Generated {} chunks in {:?} for file {:?}",
                        chunks.len(),
                        chunk_duration,
                        file_path
                    );
                    // Log first few chunk details for debugging
                    for (i, chunk) in chunks.iter().take(3).enumerate() {
                        info!(
                            "   Chunk {}: {} chars, symbol: {:?}",
                            i,
                            chunk.code.len(),
                            chunk.symbol
                        );
                    }
                    if chunks.len() > 3 {
                        info!("   ... and {} more chunks", chunks.len() - 3);
                    }
                    chunks
                }
                Err(e) => {
                    error!("❌ Failed to chunk file {:?}: {}", file_path, e);
                    return Err(e);
                }
            }
        };

        // Process chunks in batches
        let chunk_batches: Vec<Vec<Chunk>> = chunks
            .chunks(self.config.batch_size)
            .map(|chunk_slice| {
                chunk_slice
                    .iter()
                    .map(convert_chunker_to_proto_chunk)
                    .collect()
            })
            .collect();

        info!(
            "📦 Processing {} chunk batches (batch size: {}) for file {:?}",
            chunk_batches.len(),
            self.config.batch_size,
            file_path
        );

        let mut total_embeddings = 0;
        for (batch_idx, batch) in chunk_batches.iter().enumerate() {
            let batch_start = std::time::Instant::now();
            info!(
                "🔄 Processing batch {}/{} with {} chunks for file {:?}",
                batch_idx + 1,
                chunk_batches.len(),
                batch.len(),
                file_path
            );
            match self.process_chunk_batch(batch.clone()).await {
                Ok(()) => {
                    total_embeddings += batch.len();
                    let batch_duration = batch_start.elapsed();
                    info!(
                        "✅ Batch {}/{} processed ({} chunks) in {:?} for file {:?}",
                        batch_idx + 1,
                        chunk_batches.len(),
                        batch.len(),
                        batch_duration,
                        file_path
                    );
                }
                Err(e) => {
                    error!(
                        "❌ Failed to process batch {}/{} for file {:?}: {}",
                        batch_idx + 1,
                        chunk_batches.len(),
                        file_path,
                        e
                    );
                    return Err(e);
                }
            }
        }

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.files_processed += 1;
        stats.chunks_created += chunks.len() as u64;
        stats.bytes_processed += file_size;

        let total_duration = start_time.elapsed();
        info!(
            "✅ File processing complete: {:?} - {} chunks, {} embeddings in {:?}",
            file_path,
            chunks.len(),
            total_embeddings,
            total_duration
        );

        Ok(())
    }

    /// Process a batch of chunks: generate embeddings and index them
    async fn process_chunk_batch(&self, chunks: Vec<Chunk>) -> Result<()> {
        let start_time = std::time::Instant::now();
        debug!("🔄 Processing batch of {} chunks", chunks.len());

        // Extract text content for embedding
        let texts: Vec<String> = chunks.iter().map(|chunk| chunk.code.clone()).collect();

        // Generate embeddings
        let embed_start = std::time::Instant::now();
        let embeddings = match self.embedder.embed_batch(&texts).await {
            Ok(embeddings) => {
                let embed_duration = embed_start.elapsed();
                debug!(
                    "🤖 Generated {} embeddings in {:?}",
                    embeddings.len(),
                    embed_duration
                );
                embeddings
            }
            Err(e) => {
                error!("❌ Failed to generate embeddings for batch: {}", e);
                return Err(e);
            }
        };

        if embeddings.len() != chunks.len() {
            let error_msg = format!(
                "Embedding count mismatch: expected {}, got {}",
                chunks.len(),
                embeddings.len()
            );
            error!("❌ {}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        // Index each chunk with its embedding
        let index_start = std::time::Instant::now();
        let mut index_service = self.index_service.write().await;
        let mut indexed_count = 0;

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            let mut payload = qdrant_client::Payload::new();
            payload.insert("path", chunk.path.clone());
            payload.insert("lang", chunk.lang.clone());
            payload.insert("rev", chunk.rev.clone());
            payload.insert("size", chunk.size as i64);
            payload.insert("code", chunk.code.clone());
            // Add branch information for better search filtering
            payload.insert("branch", chunk.rev.clone());

            if let Some(symbol) = &chunk.symbol {
                payload.insert("symbol", symbol.clone());
            }

            if let Some(summary) = &chunk.summary {
                payload.insert("summary", summary.clone());
            }

            match index_service
                .add_embedding(&chunk.id, embedding.clone(), payload)
                .await
            {
                Ok(()) => {
                    indexed_count += 1;
                }
                Err(e) => {
                    error!("❌ Failed to index chunk {}: {}", chunk.id, e);
                    let mut stats = self.stats.write().await;
                    stats.errors_encountered += 1;
                    return Err(e.into());
                }
            }
        }

        let index_duration = index_start.elapsed();
        debug!(
            "🗂️  Indexed {} chunks in {:?}",
            indexed_count, index_duration
        );

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.embeddings_generated += embeddings.len() as u64;

        let total_duration = start_time.elapsed();
        debug!(
            "✅ Batch processing complete - {} chunks processed in {:?}",
            chunks.len(),
            total_duration
        );
        Ok(())
    }

    /// Process multiple files concurrently
    pub async fn process_files(&self, file_paths: Vec<PathBuf>) -> Result<()> {
        let start_time = std::time::Instant::now();
        info!(
            "🚀 Starting concurrent processing of {} files",
            file_paths.len()
        );

        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_files,
        ));
        let mut tasks = Vec::new();

        for (idx, file_path) in file_paths.iter().enumerate() {
            let semaphore = semaphore.clone();
            let pipeline = self.clone_for_task();
            let file_path = file_path.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                debug!(
                    "🔄 Starting processing of file {}: {:?}",
                    idx + 1,
                    file_path
                );
                let result = pipeline.process_file(&file_path).await;
                if let Err(ref e) = result {
                    error!(
                        "❌ Failed to process file {}: {:?} - {}",
                        idx + 1,
                        file_path,
                        e
                    );
                } else {
                    debug!(
                        "✅ Completed processing of file {}: {:?}",
                        idx + 1,
                        file_path
                    );
                }
                result
            });

            tasks.push(task);
        }

        info!(
            "⏳ Waiting for {} concurrent tasks to complete...",
            tasks.len()
        );

        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;

        // Check for errors
        let mut error_count = 0;
        let mut success_count = 0;
        for (idx, result) in results.iter().enumerate() {
            match result {
                Ok(Ok(())) => {
                    success_count += 1;
                }
                Ok(Err(e)) => {
                    error!("❌ File processing error for task {}: {}", idx + 1, e);
                    error_count += 1;
                }
                Err(e) => {
                    error!("❌ Task join error for task {}: {}", idx + 1, e);
                    error_count += 1;
                }
            }
        }

        let total_duration = start_time.elapsed();
        if error_count > 0 {
            warn!(
                "⚠️  Concurrent processing completed with {} successes and {} errors in {:?}",
                success_count, error_count, total_duration
            );
        } else {
            info!(
                "✅ All {} files processed successfully in {:?}",
                file_paths.len(),
                total_duration
            );
        }

        Ok(())
    }

    /// Get current pipeline statistics
    pub async fn get_stats(&self) -> PipelineStats {
        self.stats.read().await.clone()
    }

    /// Reset pipeline statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = PipelineStats::default();
    }

    // Helper methods

    fn get_language_from_path(&self, path: &Path) -> String {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_else(|| "text".to_string())
    }

    fn generate_file_hash(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Clone the pipeline for use in async tasks
    /// This creates a lightweight clone that shares the same underlying
    /// services
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            chunker: self.chunker.clone(),
            embedder: self.embedder.clone(),
            index_service: self.index_service.clone(),
            stats: self.stats.clone(),
            file_watcher: None,
            event_receiver: None,
            gitignore: self.gitignore.clone(),
            walker: self.walker.clone(),
        }
    }
}

/// Convert chunker::Chunk to proto::Chunk
fn convert_chunker_to_proto_chunk(chunk: &chunker::Chunk) -> Chunk {
    Chunk {
        id: chunk.id.clone(),
        path: chunk.path.clone(),
        lang: chunk.lang.clone(),
        symbol: chunk.symbol.clone(),
        rev: chunk.rev.clone(),
        size: chunk.size,
        code: chunk.code.clone(),
        summary: chunk.summary.clone(),
        embedding: None, // Will be filled later by the embedder
    }
}
