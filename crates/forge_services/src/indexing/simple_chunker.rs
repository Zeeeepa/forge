//! Simple text-based chunker implementation

use std::path::Path;

use anyhow::Result;
use forge_domain::{ChunkingConfig, CodeChunk, Chunker, detect_language_from_extension};

/// A simple chunker that splits content by lines and size
pub struct SimpleChunker {
    supported_languages: Vec<String>,
}

impl Default for SimpleChunker {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleChunker {
    pub fn new() -> Self {
        Self {
            supported_languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "java".to_string(),
                "go".to_string(),
                "c".to_string(),
                "cpp".to_string(),
                "markdown".to_string(),
                "text".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "toml".to_string(),
                "xml".to_string(),
                "html".to_string(),
                "css".to_string(),
                "scss".to_string(),
            ],
        }
    }
}

#[async_trait::async_trait]
impl Chunker for SimpleChunker {
    async fn chunk_file(
        &self,
        path: &str,
        content: &str,
        language: &str,
        revision: &str,
        config: &ChunkingConfig,
    ) -> Result<Vec<CodeChunk>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Ok(chunks);
        }

        let mut start_line = 0;
        let mut chunk_id = 0;

        while start_line < lines.len() {
            let mut end_line = start_line;
            let mut current_chars = 0;

            // Build chunk respecting character limits
            while end_line < lines.len()
                && current_chars + lines[end_line].len() <= config.max_chunk_size
            {
                current_chars += lines[end_line].len() + 1; // +1 for newline
                end_line += 1;
            }

            // Ensure we have at least one line
            if end_line == start_line {
                end_line = start_line + 1;
            }

            // Apply minimum chunk size constraint
            if current_chars < config.min_chunk_size && end_line < lines.len() {
                continue;
            }

            let chunk_content = lines[start_line..end_line].join("\n");

            chunks.push(CodeChunk::new(
                format!("{path}:{chunk_id}"),
                path.to_string(),
                language.to_string(),
                revision.to_string(),
                chunk_content,
                start_line + 1, // 1-based line numbers
                end_line,
            ));

            chunk_id += 1;

            // Move to next chunk with overlap
            let overlap_lines = config.overlap_size / 50; // Rough conversion from chars to lines
            start_line = if end_line > overlap_lines {
                end_line - overlap_lines
            } else {
                end_line
            };
        }

        Ok(chunks)
    }

    fn supported_languages(&self) -> &[String] {
        &self.supported_languages
    }

    fn detect_language(&self, path: &Path) -> Option<String> {
        detect_language_from_extension(path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use forge_domain::ChunkingStrategy;
    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn test_simple_chunker() {
        let chunker = SimpleChunker::new();
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let path = "test.txt";
        let language = "text";
        let revision = "abc123";
        let config = ChunkingConfig {
            max_chunk_size: 20,
            min_chunk_size: 5,
            overlap_size: 10,
            strategy: ChunkingStrategy::SizeBased,
            semantic_languages: vec![],
        };

        let actual = chunker
            .chunk_file(path, content, language, revision, &config)
            .await
            .unwrap();

        assert!(!actual.is_empty());
        assert_eq!(actual[0].path, "test.txt");
        assert_eq!(actual[0].language, "text");
        assert_eq!(actual[0].revision, "abc123");
    }

    #[test]
    fn test_language_detection() {
        let chunker = SimpleChunker::new();

        let fixtures = vec![
            ("test.rs", Some("rust".to_string())),
            ("test.py", Some("python".to_string())),
            ("test.js", Some("javascript".to_string())),
            ("test.unknown", None),
        ];

        for (path_str, expected) in fixtures {
            let path = PathBuf::from(path_str);
            let actual = chunker.detect_language(&path);
            assert_eq!(actual, expected, "Failed for path: {}", path_str);
        }
    }

    #[test]
    fn test_supported_languages() {
        let chunker = SimpleChunker::new();
        let languages = chunker.supported_languages();

        assert!(languages.contains(&"rust".to_string()));
        assert!(languages.contains(&"python".to_string()));
        assert!(languages.len() > 5);
    }
}
