//! Domain interfaces and types for code chunking

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;

use crate::{ChunkingConfig, CodeChunk};

/// Trait for chunking code into semantic pieces
#[async_trait]
pub trait Chunker: Send + Sync {
    /// Chunk a file's content into semantic pieces
    async fn chunk_file(
        &self,
        path: &str,
        content: &str,
        language: &str,
        revision: &str,
        config: &ChunkingConfig,
    ) -> Result<Vec<CodeChunk>>;

    /// Get supported languages for semantic chunking
    fn supported_languages(&self) -> &[String];

    /// Detect programming language from file extension
    fn detect_language(&self, path: &Path) -> Option<String>;
}

/// Utility functions for language detection
pub fn detect_language_from_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_lowercase().as_str() {
            "rs" => Some("rust".to_string()),
            "py" => Some("python".to_string()),
            "js" => Some("javascript".to_string()),
            "ts" => Some("typescript".to_string()),
            "java" => Some("java".to_string()),
            "go" => Some("go".to_string()),
            "c" => Some("c".to_string()),
            "cpp" | "cc" | "cxx" => Some("cpp".to_string()),
            "md" => Some("markdown".to_string()),
            "txt" => Some("text".to_string()),
            "json" => Some("json".to_string()),
            "yaml" | "yml" => Some("yaml".to_string()),
            "toml" => Some("toml".to_string()),
            "xml" => Some("xml".to_string()),
            "html" => Some("html".to_string()),
            "css" => Some("css".to_string()),
            "scss" | "sass" => Some("scss".to_string()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_language_detection() {
        let fixtures = vec![
            ("test.rs", Some("rust".to_string())),
            ("test.py", Some("python".to_string())),
            ("test.js", Some("javascript".to_string())),
            ("test.ts", Some("typescript".to_string())),
            ("test.java", Some("java".to_string())),
            ("test.go", Some("go".to_string())),
            ("test.c", Some("c".to_string())),
            ("test.cpp", Some("cpp".to_string())),
            ("test.cc", Some("cpp".to_string())),
            ("test.cxx", Some("cpp".to_string())),
            ("test.md", Some("markdown".to_string())),
            ("test.txt", Some("text".to_string())),
            ("test.json", Some("json".to_string())),
            ("test.yaml", Some("yaml".to_string())),
            ("test.yml", Some("yaml".to_string())),
            ("test.toml", Some("toml".to_string())),
            ("test.xml", Some("xml".to_string())),
            ("test.html", Some("html".to_string())),
            ("test.css", Some("css".to_string())),
            ("test.scss", Some("scss".to_string())),
            ("test.sass", Some("scss".to_string())),
            ("test.unknown", None),
        ];

        for (path_str, expected) in fixtures {
            let path = PathBuf::from(path_str);
            let actual = detect_language_from_extension(&path);
            assert_eq!(actual, expected, "Failed for path: {}", path_str);
        }
    }
}
