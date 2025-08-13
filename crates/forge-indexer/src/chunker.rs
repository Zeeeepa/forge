use anyhow::Result;
use text_splitter::CodeSplitter;
use tracing::debug;

// Re-export public types
pub use crate::proto::Chunk;

pub struct Chunker {
    pub max_chunk_size: usize,
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunker {
    pub fn new() -> Self {
        Self {
            max_chunk_size: 500,
        }
    }

    /// Chunk a file using text_splitter::CodeSplitter when possible, otherwise fallback to simple windows.
    pub fn chunk_file(
        &mut self,
        path: &str,
        content: &str,
        lang: &str,
        rev: &str,
    ) -> Result<Vec<Chunk>> {
        // Try to construct a CodeSplitter using the language if available
        let maybe_splitter = match lang {
            "rust" => Some(CodeSplitter::new(tree_sitter_rust::LANGUAGE, self.max_chunk_size)),
            "python" => Some(CodeSplitter::new(tree_sitter_python::LANGUAGE, self.max_chunk_size)),
            "javascript" | "typescript" => Some(CodeSplitter::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT, self.max_chunk_size)),
            "go" => Some(CodeSplitter::new(tree_sitter_go::LANGUAGE, self.max_chunk_size)),
            "java" => Some(CodeSplitter::new(tree_sitter_java::LANGUAGE, self.max_chunk_size)),
            "cpp" | "c" => Some(CodeSplitter::new(tree_sitter_cpp::LANGUAGE, self.max_chunk_size)),
            "css" => Some(CodeSplitter::new(tree_sitter_css::LANGUAGE, self.max_chunk_size)),
            "ruby" => Some(CodeSplitter::new(tree_sitter_ruby::LANGUAGE, self.max_chunk_size)),
            _ => None,
        };

        let mut chunks = Vec::new();

        if let Some(splitter_res) = maybe_splitter {
            match splitter_res {
                Ok(splitter) => {
                    // Use text-splitter's built-in chunking which handles overlaps and min sizes
                    for (i, piece) in splitter.chunks(content).into_iter().enumerate() {
                        let start = content.find(piece).unwrap_or(0);
                        let end = start + piece.len();
                        let chunk = Chunk {
                            id: format!("{}:{}:{}:{}", path, start, end, rev),
                            path: path.to_string(),
                            lang: lang.to_string(),
                            symbol: None,
                            rev: rev.to_string(),
                            size: piece.len(),
                            code: piece.to_string(),
                            summary: None,
                            embedding: None,
                        };
                        debug!("Created chunk {} for {}: {} chars", i, path, piece.len());
                        chunks.push(chunk);
                    }
                }
                Err(e) => {
                    debug!("Failed to create CodeSplitter for {}: {:?}", lang, e);
                    // Fall back to simple window splitting
                    debug!("Falling back to window splitting for {}", path);
                    chunks = self.create_chunks_with_windows(path, content, lang, rev);
                }
            }
        } else {
            // No splitter available, fall back to simple window splitting
            chunks = self.create_chunks_with_windows(path, content, lang, rev);
        }

        Ok(chunks)
    }

    /// Create chunks using simple window splitting
    fn create_chunks_with_windows(&self, path: &str, content: &str, lang: &str, rev: &str) -> Vec<Chunk> {
        // For unsupported languages, just create one chunk with the entire content
        // since we're simplifying and letting text-splitter handle the complex logic
        debug!("Creating fallback chunk for {}: {} chars", path, content.len());
        vec![Chunk {
            id: format!("{}:{}:{}:{}", path, 0, content.len(), rev),
            path: path.to_string(),
            lang: lang.to_string(),
            symbol: None,
            rev: rev.to_string(),
            size: content.len(),
            code: content.to_string(),
            summary: None,
            embedding: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunker_with_rust_code() {
        let mut chunker = Chunker::new();
        let content = r#"
        fn hello_world() {
            println!("Hello, world!");
        }
        
        fn main() {
            hello_world();
        }
        "#;
        
        let chunks = chunker.chunk_file("test.rs", content, "rust", "rev1").unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.len() > 0);
        
        // Verify chunk properties
        for chunk in &chunks {
            assert!(!chunk.id.is_empty());
            assert_eq!(chunk.path, "test.rs");
            assert_eq!(chunk.lang, "rust");
            assert_eq!(chunk.rev, "rev1");
            assert!(chunk.size > 0);
            assert!(!chunk.code.is_empty());
        }
    }
    
    #[test]
    fn test_chunker_with_unsupported_language() {
        let mut chunker = Chunker::new();
        let content = "This is a test file with some content.";
        
        let chunks = chunker.chunk_file("test.txt", content, "unknown", "rev1").unwrap();
        assert!(!chunks.is_empty());
        assert_eq!(chunks.len(), 1);
        
        // Should fall back to simple window splitting
        let chunk = &chunks[0];
        assert!(!chunk.id.is_empty());
        assert_eq!(chunk.path, "test.txt");
        assert_eq!(chunk.lang, "unknown");
        assert_eq!(chunk.rev, "rev1");
        assert_eq!(chunk.size, content.len());
        assert_eq!(chunk.code, content);
    }
}