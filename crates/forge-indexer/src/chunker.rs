//! Code chunker with AST parsing support

use anyhow::Result;
use tracing::debug;
use tree_sitter::Tree;
// Re-export public types
pub use types::Chunk;

// Internal modules
mod analysis;
mod parser;
mod similarity;
mod strategies;
mod types;
mod utils;

use analysis::CodeAnalyzer;
use parser::ParserManager;
use similarity::SimilarityCalculator;
use strategies::ChunkingStrategies;
use types::CodeAnalysis;
use utils::ChunkerUtils;

pub struct Chunker {
    parser_manager: ParserManager,
    max_chunk_size: usize,
    min_chunk_size: usize,
    overlap_size: usize,
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunker {
    pub fn new() -> Self {
        Self {
            parser_manager: ParserManager::new(),
            max_chunk_size: 2000,
            min_chunk_size: 100,
            overlap_size: 200,
        }
    }

    pub fn chunk_file(
        &mut self,
        path: &str,
        content: &str,
        lang: &str,
        rev: &str,
    ) -> Result<Vec<Chunk>> {
        // Parse the file with tree-sitter if we have a parser for this language
        let tree = if self.parser_manager.has_parser(lang) {
            if let Some(parser) = self.parser_manager.get_parser(lang) {
                parser.parse(content, None)
            } else {
                // Fallback to creating a new parser instance
                if let Some(mut new_parser) = self.parser_manager.create_parser(lang) {
                    new_parser.parse(content, None)
                } else {
                    None
                }
            }
        } else {
            debug!(
                "ℹ️  Language {} not supported by tree-sitter, using fallback chunking",
                lang
            );
            None
        };

        let chunks = if let Some(tree) = tree.as_ref() {
            // Advanced semantic chunking with AST analysis
            let mut code_analysis = CodeAnalyzer::analyze_code_structure(content, lang, tree);

            // Enhance analysis with instance methods
            let additional_boundaries = self.detect_semantic_boundaries(tree, content);
            code_analysis
                .semantic_boundaries
                .extend(additional_boundaries);
            code_analysis.semantic_boundaries.sort_unstable();
            code_analysis.semantic_boundaries.dedup();

            // Extract additional symbols and keywords using instance methods
            let keywords = self.extract_keywords(content);
            let mut symbols = self.extract_symbols(content, lang);

            // Use tree-sitter for more accurate symbol collection
            let mut cursor = tree.walk();
            let mut tree_symbols = std::collections::HashSet::new();
            ChunkerUtils::collect_symbols(&mut cursor, content, &mut tree_symbols);
            symbols.extend(tree_symbols);
            symbols.sort();
            symbols.dedup();

            // Add extracted symbols to dependencies for better context
            code_analysis.dependencies.extend(keywords);
            code_analysis.dependencies.extend(symbols);
            code_analysis.dependencies.sort();
            code_analysis.dependencies.dedup();

            let semantic_chunks = ChunkingStrategies::extract_semantic_chunks(
                path,
                content,
                lang,
                rev,
                &code_analysis,
                tree,
            );

            if !semantic_chunks.is_empty() {
                semantic_chunks
            } else {
                ChunkingStrategies::extract_context_chunks(
                    path,
                    content,
                    lang,
                    rev,
                    &code_analysis,
                    tree,
                )
            }
        } else {
            // Fallback to simple chunking for unsupported languages
            ChunkingStrategies::fallback_chunking(path, content, lang, rev, tree.as_ref())
        };

        // Post-process chunks for optimization using semantic similarity with size
        // constraints
        let mut optimized_chunks =
            SimilarityCalculator::optimize_chunks(chunks, &CodeAnalysis::default())?;

        // Apply size constraints from chunker configuration with overlap consideration
        optimized_chunks
            .retain(|chunk| chunk.size >= self.min_chunk_size && chunk.size <= self.max_chunk_size);

        // Apply overlap processing if configured
        if self.overlap_size > 0 && optimized_chunks.len() > 1 {
            // Add overlap information to chunk summaries for better context
            for chunk in optimized_chunks.iter_mut() {
                if let Some(ref mut summary) = chunk.summary {
                    summary.push_str(&format!(" | Overlap: {}", self.overlap_size));
                }
                // Note: Actual overlap implementation would require content
                // modification This is a placeholder for future
                // enhancement
            }
        }

        // Generate semantic summaries for chunks that don't have them
        for chunk in &mut optimized_chunks {
            if chunk.summary.is_none() {
                chunk.summary = SimilarityCalculator::generate_semantic_summary(
                    chunk,
                    &CodeAnalysis::default(),
                );
            }
        }

        // Apply enhanced semantic similarity analysis for chunk refinement using
        // tree-sitter
        if optimized_chunks.len() > 1 {
            for i in 0..optimized_chunks.len() {
                // Calculate complexity score for better chunk ranking
                if let Some(ref tree) = tree {
                    let complexity = self.calculate_chunk_complexity(&optimized_chunks[i], tree);
                    // Add complexity info to summary
                    if let Some(ref mut summary) = optimized_chunks[i].summary {
                        summary.push_str(&format!(" | Complexity: {complexity:.2}"));
                    }
                }

                for j in (i + 1)..optimized_chunks.len() {
                    // Use tree-sitter enhanced similarity calculation for better accuracy
                    let tree_similarity =
                        SimilarityCalculator::calculate_semantic_similarity_with_tree(
                            &optimized_chunks[i],
                            &optimized_chunks[j],
                            tree.as_ref(),
                            tree.as_ref(),
                        );

                    // Also use the basic semantic similarity for comparison
                    let basic_similarity = self
                        .calculate_semantic_similarity(&optimized_chunks[i], &optimized_chunks[j]);

                    // Also calculate Jaccard similarity for comparison
                    let jaccard_sim = self
                        .calculate_jaccard_similarity(&optimized_chunks[i], &optimized_chunks[j]);

                    // Use the highest similarity score for decision making
                    let max_similarity = tree_similarity.max(basic_similarity);

                    if max_similarity > 0.8 || jaccard_sim > 0.7 {
                        // Add similarity info to chunk summaries
                        if let Some(ref mut summary) = optimized_chunks[i].summary {
                            summary.push_str(&format!(" | Similar to chunk {j} (tree: {tree_similarity:.2}, basic: {basic_similarity:.2}, jac: {jaccard_sim:.2})"));
                        }
                    }
                }
            }
        }

        Ok(optimized_chunks)
    }

    /// Detect semantic boundaries in code
    fn detect_semantic_boundaries(&self, tree: &Tree, content: &str) -> Vec<usize> {
        ChunkerUtils::detect_semantic_boundaries(tree, content)
    }

    /// Calculate semantic similarity between two chunks
    fn calculate_semantic_similarity(&self, chunk1: &Chunk, chunk2: &Chunk) -> f32 {
        SimilarityCalculator::calculate_chunk_similarity(chunk1, chunk2)
    }

    /// Extract keywords from content using enhanced analysis
    fn extract_keywords(&self, content: &str) -> Vec<String> {
        ChunkerUtils::extract_keywords(content)
            .into_iter()
            .collect()
    }

    /// Extract symbols (identifiers) from content using enhanced analysis
    fn extract_symbols(&self, content: &str, lang: &str) -> Vec<String> {
        let mut symbols = ChunkerUtils::extract_symbols(content, lang)
            .into_iter()
            .collect::<Vec<_>>();

        // Note: Tree-sitter enhanced symbol extraction would require mutable access
        // This is a design consideration for future enhancement

        symbols.sort();
        symbols.dedup();
        symbols
    }

    /// Calculate semantic complexity for better chunk scoring
    fn calculate_chunk_complexity(&self, chunk: &Chunk, tree: &Tree) -> f32 {
        ChunkerUtils::calculate_semantic_complexity(&chunk.code, tree, &chunk.code)
    }

    /// Calculate similarity between chunks using Jaccard similarity
    fn calculate_jaccard_similarity(&self, chunk1: &Chunk, chunk2: &Chunk) -> f32 {
        let keywords1 = ChunkerUtils::extract_keywords(&chunk1.code);
        let keywords2 = ChunkerUtils::extract_keywords(&chunk2.code);
        ChunkerUtils::jaccard_similarity(&keywords1, &keywords2)
    }
}
