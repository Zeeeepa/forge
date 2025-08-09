//! Semantic similarity calculations and optimization

use std::collections::HashSet;

use anyhow::Result;
use tree_sitter::{Tree, TreeCursor};

use crate::chunker::types::{Chunk, CodeAnalysis};

pub(crate) struct SimilarityCalculator;

impl SimilarityCalculator {
    /// Optimize chunks for better semantic coherence
    pub fn optimize_chunks(chunks: Vec<Chunk>, analysis: &CodeAnalysis) -> Result<Vec<Chunk>> {
        if chunks.is_empty() {
            return Ok(chunks);
        }

        let mut optimized = Vec::new();
        let mut i = 0;

        while i < chunks.len() {
            let mut current_chunk = chunks[i].clone();

            // If chunk is too small, try to merge with next chunk
            if current_chunk.size < 100 && i + 1 < chunks.len() {
                let next_chunk = &chunks[i + 1];

                // Use semantic similarity to decide if chunks should be merged
                let similarity = if current_chunk.lang == next_chunk.lang {
                    Self::calculate_chunk_similarity(&current_chunk, next_chunk)
                } else {
                    0.0
                };

                // Merge if chunks are similar enough or if total size is reasonable
                if (similarity > 0.3 || current_chunk.size + next_chunk.size < 3000)
                    && current_chunk.size + next_chunk.size < 5000
                {
                    current_chunk.code = format!("{}\n{}", current_chunk.code, next_chunk.code);
                    current_chunk.size = current_chunk.code.len();
                    current_chunk.id = format!("{}+{}", current_chunk.id, next_chunk.id);

                    // Update symbol if merging related code
                    if current_chunk.symbol.is_none() && next_chunk.symbol.is_some() {
                        current_chunk.symbol = next_chunk.symbol.clone();
                    }

                    i += 2; // Skip the next chunk as it's been merged
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }

            // Add complexity-based summary for larger chunks
            if current_chunk.size > 1000 && current_chunk.summary.is_none() {
                current_chunk.summary = Self::generate_semantic_summary(&current_chunk, analysis);
            }

            optimized.push(current_chunk);
        }

        Ok(optimized)
    }

    /// Calculate semantic similarity between two chunks using our implemented
    /// functions
    pub fn calculate_chunk_similarity(chunk1: &Chunk, chunk2: &Chunk) -> f32 {
        // Extract keywords and symbols from both chunks
        let keywords1 = Self::extract_keywords_static(&chunk1.code);
        let keywords2 = Self::extract_keywords_static(&chunk2.code);

        let symbols1 = Self::extract_symbols_static(&chunk1.code, &chunk1.lang);
        let symbols2 = Self::extract_symbols_static(&chunk2.code, &chunk2.lang);

        // Calculate keyword similarity (Jaccard similarity)
        let keyword_similarity = Self::jaccard_similarity_static(&keywords1, &keywords2);

        // Calculate symbol similarity
        let symbol_similarity = Self::jaccard_similarity_static(&symbols1, &symbols2);

        // Weight the similarities (symbols are more important for semantic meaning)
        0.3 * keyword_similarity + 0.7 * symbol_similarity
    }

    /// Calculate semantic similarity between two chunks with tree-sitter
    /// support
    pub fn calculate_semantic_similarity_with_tree(
        chunk1: &Chunk,
        chunk2: &Chunk,
        tree1: Option<&Tree>,
        tree2: Option<&Tree>,
    ) -> f32 {
        // If we have tree-sitter trees, use them for more accurate analysis
        if let (Some(tree1), Some(tree2)) = (tree1, tree2) {
            let keywords1 = Self::extract_keywords_from_tree(&chunk1.code, tree1);
            let keywords2 = Self::extract_keywords_from_tree(&chunk2.code, tree2);

            let symbols1 = Self::extract_symbols_from_tree(&chunk1.code, tree1);
            let symbols2 = Self::extract_symbols_from_tree(&chunk2.code, tree2);

            let keyword_similarity = Self::jaccard_similarity_static(&keywords1, &keywords2);
            let symbol_similarity = Self::jaccard_similarity_static(&symbols1, &symbols2);

            0.3 * keyword_similarity + 0.7 * symbol_similarity
        } else {
            // Fallback to static analysis
            Self::calculate_chunk_similarity(chunk1, chunk2)
        }
    }

    /// Generate semantic summary based on code analysis
    pub fn generate_semantic_summary(chunk: &Chunk, analysis: &CodeAnalysis) -> Option<String> {
        if let Some(symbol) = &chunk.symbol {
            // Find the symbol in our analysis
            let symbol_info = analysis.symbols.iter().find(|s| s.name == *symbol);

            if let Some(info) = symbol_info {
                let complexity_desc = match info.complexity {
                    1..=2 => "simple",
                    3..=5 => "moderate",
                    6..=10 => "complex",
                    _ => "highly complex",
                };

                return Some(format!(
                    "{} {} ({})",
                    complexity_desc,
                    info.symbol_type.replace("_", " "),
                    symbol
                ));
            }
        }

        // Fallback summary based on content analysis
        let lines = chunk.code.lines().count();
        if lines > 50 {
            Some(format!("Large code block ({lines} lines)"))
        } else if chunk.code.contains("fn ") || chunk.code.contains("function ") {
            Some("Function implementation".to_string())
        } else if chunk.code.contains("struct ") || chunk.code.contains("class ") {
            Some("Type definition".to_string())
        } else {
            Some("Code block".to_string())
        }
    }

    /// Static version of extract_keywords for use in optimization
    pub fn extract_keywords_static(content: &str) -> HashSet<String> {
        let mut keywords = HashSet::new();

        // Common programming keywords
        let common_keywords = [
            "function",
            "class",
            "struct",
            "enum",
            "trait",
            "interface",
            "public",
            "private",
            "protected",
            "static",
            "const",
            "mut",
            "async",
            "await",
            "return",
            "yield",
            "throw",
            "catch",
            "if",
            "else",
            "while",
            "for",
            "match",
            "switch",
            "case",
            "import",
            "export",
            "use",
            "include",
            "require",
            "new",
            "delete",
            "malloc",
            "free",
            "clone",
            "impl",
            "extends",
            "implements",
            "inherits",
        ];

        for word in content.split_whitespace() {
            let clean_word = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if clean_word.len() > 2
                && (common_keywords.contains(&clean_word.as_str())
                    || clean_word.chars().any(|c| c.is_uppercase()))
            {
                keywords.insert(clean_word);
            }
        }

        keywords
    }

    /// Static version of extract_symbols for use in optimization
    pub fn extract_symbols_static(content: &str, _lang: &str) -> HashSet<String> {
        let mut symbols = HashSet::new();

        // Fallback: extract camelCase and snake_case identifiers
        for word in content.split_whitespace() {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if Self::is_likely_identifier_static(clean_word) {
                symbols.insert(clean_word.to_string());
            }
        }

        symbols
    }

    /// Extract keywords from tree-sitter tree
    pub fn extract_keywords_from_tree(content: &str, tree: &Tree) -> HashSet<String> {
        let mut keywords = HashSet::new();
        let mut cursor = tree.walk();

        Self::collect_keywords_recursive(&mut cursor, content, &mut keywords);
        keywords
    }

    /// Extract symbols from tree-sitter tree
    pub fn extract_symbols_from_tree(content: &str, tree: &Tree) -> HashSet<String> {
        let mut symbols = HashSet::new();
        let mut cursor = tree.walk();

        Self::collect_symbols_recursive(&mut cursor, content, &mut symbols);
        symbols
    }

    /// Static version of is_likely_identifier for use in optimization
    pub fn is_likely_identifier_static(word: &str) -> bool {
        if word.len() < 2 {
            return false;
        }

        // Must start with letter or underscore
        if !word.chars().next().unwrap().is_alphabetic() && !word.starts_with('_') {
            return false;
        }

        // Must contain only alphanumeric and underscores
        if !word.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return false;
        }

        // Likely identifier patterns
        word.contains('_') || // snake_case
        word.chars().any(|c| c.is_uppercase()) || // camelCase or PascalCase
        word.len() > 3 // longer words are more likely to be meaningful
    }

    /// Static version of jaccard_similarity for use in optimization
    pub fn jaccard_similarity_static(set1: &HashSet<String>, set2: &HashSet<String>) -> f32 {
        if set1.is_empty() && set2.is_empty() {
            return 1.0;
        }

        let intersection = set1.intersection(set2).count();
        let union = set1.union(set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    fn collect_keywords_recursive(
        cursor: &mut TreeCursor,
        content: &str,
        keywords: &mut HashSet<String>,
    ) {
        let node = cursor.node();

        // Collect language keywords
        match node.kind() {
            "function" | "fn" | "def" | "class" | "struct" | "enum" | "trait" | "interface"
            | "public" | "private" | "protected" | "static" | "const" | "mut" | "async"
            | "await" | "return" | "yield" | "throw" | "catch" | "if" | "else" | "while"
            | "for" | "match" | "switch" | "case" | "import" | "export" | "use" | "include"
            | "require" | "new" | "delete" | "impl" | "extends" | "implements" => {
                keywords.insert(node.kind().to_string());
            }
            _ => {}
        }

        // Traverse children
        if cursor.goto_first_child() {
            loop {
                Self::collect_keywords_recursive(cursor, content, keywords);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn collect_symbols_recursive(
        cursor: &mut TreeCursor,
        content: &str,
        symbols: &mut HashSet<String>,
    ) {
        let node = cursor.node();

        if node.kind() == "identifier" {
            let symbol = &content[node.byte_range()];
            if symbol.len() > 1 {
                symbols.insert(symbol.to_string());
            }
        }

        // Traverse children
        if cursor.goto_first_child() {
            loop {
                Self::collect_symbols_recursive(cursor, content, symbols);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}
