//! File watcher service using the notify crate

use std::path::Path;

use anyhow::Result;
use notify::{Event, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
}

impl FileWatcher {
    pub fn new() -> Result<(Self, mpsc::Receiver<Event>)> {
        info!("🔧 Initializing file watcher...");

        let (tx, rx) = mpsc::channel(1024);
        let watcher = recommended_watcher(move |res| match res {
            Ok(event) => {
                debug!("📨 File system event detected: {:?}", event);
                if let Err(e) = tx.blocking_send(event) {
                    error!("❌ Failed to send file event: {}", e);
                }
            }
            Err(e) => {
                error!("❌ File system watch error: {}", e);
            }
        })?;

        info!("✅ File watcher initialized successfully");
        Ok((Self { watcher }, rx))
    }

    pub fn watch_directory(&mut self, path: &Path) -> Result<()> {
        info!("👀 Setting up directory watch for: {:?}", path);

        if !path.exists() {
            error!("❌ Watch path does not exist: {:?}", path);
            return Err(anyhow::anyhow!("Watch path does not exist: {:?}", path));
        }

        if !path.is_dir() {
            error!("❌ Watch path is not a directory: {:?}", path);
            return Err(anyhow::anyhow!("Watch path is not a directory: {:?}", path));
        }

        // Watch the directory recursively
        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| anyhow::anyhow!("Failed to watch directory: {}", e))?;

        info!("✅ Successfully watching directory: {:?}", path);
        Ok(())
    }
}
