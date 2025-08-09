//! Search service with hybrid search capabilities

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use forge_domain::{
    CodeChunk, MatchType, SearchContext, SearchMode, SearchOptions, SearchQuery, SearchResult,
    SearchResults, SearchStats, SortBy,
};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::indexing::Embedder;
use crate::search::{quick_semantic_search, quick_keyword_search, quick_hybrid_search, calculate_relevance_score, extract_function_names};
use crate::vector_store::SharedVectorStore;

/// Service for searching indexed codebases with semantic and keyword
/// capabilities
pub struct SearchService {
    vector_store: SharedVectorStore,
    embedder: Arc<RwLock<Box<dyn Embedder>>>,
}

impl SearchService {
    /// Create a new search service
    pub fn new(vector_store: SharedVectorStore, embedder: Box<dyn Embedder>) -> Self {
        Self { vector_store, embedder: Arc::new(RwLock::new(embedder)) }
    }

    /// Quick semantic search with default settings
    pub async fn quick_semantic(&self, query: impl Into<String>, limit: usize) -> Result<SearchResults> {
        let search_query = quick_semantic_search(query, limit);
        self.search(search_query).await
    }

    /// Quick keyword search with default settings
    pub async fn quick_keyword(&self, query: impl Into<String>, limit: usize) -> Result<SearchResults> {
        let search_query = quick_keyword_search(query, limit);
        self.search(search_query).await
    }

    /// Quick hybrid search with default settings
    pub async fn quick_hybrid(&self, query: impl Into<String>, limit: usize) -> Result<SearchResults> {
        let search_query = quick_hybrid_search(query, limit);
        self.search(search_query).await
    }

    /// Search for functions in the codebase
    pub async fn search_functions(&self, query: impl Into<String>, language: &str, limit: usize) -> Result<SearchResults> {
        let query_str = query.into();
        
        // First, do a normal search to get relevant chunks
        let mut search_query = quick_semantic_search(&query_str, limit * 2);
        search_query.filters.languages = vec![language.to_string()];
        
        let mut results = self.search(search_query).await?;
        
        // Filter results to only include chunks that contain functions matching the query
        results.chunks.retain(|result| {
            let functions = extract_function_names(&result.chunk.content, language);
            functions.iter().any(|func| {
                func.to_lowercase().contains(&query_str.to_lowercase())
            })
        });
        
        // Limit the results
        results.chunks.truncate(limit);
        results.total_matches = results.chunks.len();
        
        Ok(results)
    }

    /// Execute a search query
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let start_time = Instant::now();
        info!("Executing search query: '{}'", query.query);

        let results = match query.mode {
            SearchMode::Semantic => self.semantic_search(&query).await?,
            SearchMode::Keyword => self.keyword_search(&query).await?,
            SearchMode::Hybrid { semantic_weight, keyword_weight } => {
                self.hybrid_search(&query, semantic_weight, keyword_weight)
                    .await?
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Sort results
        let mut sorted_results = results;
        self.sort_results(&mut sorted_results, &query.options.sort_by);

        // Apply limit
        sorted_results.truncate(query.limit);

        // Post-process results
        let processed_results = self
            .post_process_results(sorted_results, &query.options)
            .await?;

        // Generate statistics
        let stats = self.generate_stats(&processed_results);

        // Generate suggestions (simple implementation)
        let suggestions = self.generate_suggestions(&query, &processed_results);

        // Store the length before moving processed_results
        let total_matches = processed_results.len();

        Ok(SearchResults {
            query: query.clone(),
            chunks: processed_results,
            total_matches, // Use the stored value
            execution_time_ms: execution_time,
            stats,
            suggestions,
        })
    }

    /// Perform semantic search using vector embeddings
    async fn semantic_search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        debug!("Performing semantic search");

        // Generate embedding for query
        let embedder = self.embedder.read().await;
        let query_embedding = embedder
            .embed_text(&query.query)
            .await
            .context("Failed to generate query embedding")?;

        // Search vector store
        let store = self.vector_store.read().await;
        let vector_results = store
            .search(
                "codebase", // TODO: Make collection name configurable
                &query_embedding,
                query.limit * 2, // Get more results to allow for filtering
                Some(&query.filters),
            )
            .await
            .context("Vector search failed")?;

        // Convert to search results
        let results: Vec<SearchResult> = vector_results
            .into_iter()
            .filter(|result| result.score >= query.similarity_threshold)
            .map(|result| SearchResult {
                chunk: result.chunk,
                score: result.score,
                match_type: MatchType::Semantic,
                highlighted_content: None, // Will be added in post-processing
                context: None,             // Will be added in post-processing
                explanation: Some(format!("Semantic similarity: {:.3}", result.score)),
            })
            .collect();

        debug!("Semantic search found {} results", results.len());
        Ok(results)
    }

    /// Perform keyword search using text matching
    async fn keyword_search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        debug!("Performing keyword search");

        // For now, we'll implement a simple approach by searching through all chunks
        // In a production system, this would use a dedicated text index like
        // Elasticsearch

        let store = self.vector_store.read().await;

        // Get all chunks (this is inefficient but works for demo)
        // In practice, we'd use a text search index
        let all_results = store
            .search(
                "codebase",
                &vec![0.0; 1536], // Dummy embedding - we'll filter by content
                10000,            // Large limit to get all
                Some(&query.filters),
            )
            .await?;

        let query_terms = self.extract_keywords(&query.query);
        let mut results = Vec::new();

        for vector_result in all_results {
            let score = self.calculate_keyword_score(&vector_result.chunk.content, &query_terms);
            if score > 0.0 {
                let has_exact_match =
                    self.has_exact_match(&vector_result.chunk.content, &query.query);
                results.push(SearchResult {
                    chunk: vector_result.chunk,
                    score,
                    match_type: if has_exact_match {
                        MatchType::ExactKeyword
                    } else {
                        MatchType::PartialKeyword
                    },
                    highlighted_content: None,
                    context: None,
                    explanation: Some(format!("Keyword match score: {score:.3}")),
                });
            }
        }

        debug!("Keyword search found {} results", results.len());
        Ok(results)
    }

    /// Perform hybrid search combining semantic and keyword approaches
    async fn hybrid_search(
        &self,
        query: &SearchQuery,
        semantic_weight: f32,
        keyword_weight: f32,
    ) -> Result<Vec<SearchResult>> {
        debug!(
            "Performing hybrid search with weights: semantic={}, keyword={}",
            semantic_weight, keyword_weight
        );

        // Get semantic results
        let semantic_results = self.semantic_search(query).await?;

        // Get keyword results
        let keyword_results = self.keyword_search(query).await?;

        // Combine and re-score results
        let mut combined_results = HashMap::new();

        // Add semantic results
        for result in semantic_results {
            let chunk_id = result.chunk.id.clone();
            let weighted_score = result.score * semantic_weight;
            combined_results.insert(
                chunk_id,
                (result, weighted_score, vec![MatchType::Semantic]),
            );
        }

        // Add keyword results (combining scores if chunk already exists)
        for result in keyword_results {
            let chunk_id = result.chunk.id.clone();
            let weighted_score = result.score * keyword_weight;

            if let Some((existing_result, existing_score, match_types)) =
                combined_results.get_mut(&chunk_id)
            {
                // Combine scores and match types
                let combined_score = *existing_score + weighted_score;
                *existing_score = combined_score;
                match_types.push(result.match_type.clone());
                existing_result.match_type = MatchType::Hybrid;
                existing_result.score = combined_score;
            } else {
                let match_type = result.match_type.clone();
                combined_results.insert(chunk_id, (result, weighted_score, vec![match_type]));
            }
        }

        // Convert back to vector and sort by combined score
        let mut results: Vec<SearchResult> = combined_results
            .into_values()
            .map(|(result, _, _)| result)
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!("Hybrid search found {} combined results", results.len());
        Ok(results)
    }

    /// Extract keywords from query text
    fn extract_keywords(&self, query: &str) -> Vec<String> {
        query
            .split_whitespace()
            .map(|word| {
                word.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|word| !word.is_empty() && word.len() > 2) // Filter out very short words
            .collect()
    }

    /// Calculate enhanced relevance score using semantic and keyword signals
    fn calculate_enhanced_score(
        &self,
        semantic_score: f32,
        keyword_score: f32,
        path_match: bool,
        symbol_match: bool,
    ) -> f32 {
        calculate_relevance_score(semantic_score, keyword_score, path_match, symbol_match)
    }

    /// Calculate keyword matching score
    fn calculate_keyword_score(&self, content: &str, keywords: &[String]) -> f32 {
        if keywords.is_empty() {
            return 0.0;
        }

        let content_lower = content.to_lowercase();
        let mut score = 0.0;
        let mut matches = 0;

        for keyword in keywords {
            let keyword_count = content_lower.matches(keyword).count();
            if keyword_count > 0 {
                matches += 1;
                // Score based on frequency and keyword length
                score += (keyword_count as f32) * (keyword.len() as f32 / 10.0);
            }
        }

        // Normalize by number of keywords and content length
        if matches > 0 {
            let coverage = matches as f32 / keywords.len() as f32;
            let density = score / (content.len() as f32 / 1000.0); // Per 1000 chars
            coverage * density.min(1.0) // Cap density at 1.0
        } else {
            0.0
        }
    }

    /// Check if content has exact match for query
    fn has_exact_match(&self, content: &str, query: &str) -> bool {
        content.to_lowercase().contains(&query.to_lowercase())
    }

    /// Sort results according to sort criteria
    fn sort_results(&self, results: &mut [SearchResult], sort_by: &SortBy) {
        match sort_by {
            SortBy::Relevance => {
                // Already sorted by score in most cases
                results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::DateDesc => {
                results.sort_by(|a, b| {
                    b.chunk
                        .metadata
                        .modified_at
                        .cmp(&a.chunk.metadata.modified_at)
                });
            }
            SortBy::DateAsc => {
                results.sort_by(|a, b| {
                    a.chunk
                        .metadata
                        .modified_at
                        .cmp(&b.chunk.metadata.modified_at)
                });
            }
            SortBy::Path => {
                results.sort_by(|a, b| a.chunk.path.cmp(&b.chunk.path));
            }
            SortBy::Complexity => {
                results.sort_by(|a, b| {
                    b.chunk
                        .metadata
                        .complexity_score
                        .unwrap_or(0.0)
                        .partial_cmp(&a.chunk.metadata.complexity_score.unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::Size => {
                results.sort_by(|a, b| b.chunk.size.cmp(&a.chunk.size));
            }
        }
    }

    /// Post-process results (add highlighting, context, etc.)
    async fn post_process_results(
        &self,
        mut results: Vec<SearchResult>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        for result in &mut results {
            // Add content highlighting if requested
            if options.highlight_matches {
                result.highlighted_content =
                    Some(self.highlight_matches(&result.chunk.content, &result.chunk.content)); // Simplified
            }

            // Add context if requested
            if options.include_context {
                result.context = self
                    .generate_context(&result.chunk, options.context_lines)
                    .await;
            }

            // Truncate content if max length is specified
            if let Some(max_length) = options.max_content_length
                && result.chunk.content.len() > max_length {
                    let mut truncated = result.chunk.content.clone();
                    truncated.truncate(max_length);
                    truncated.push_str("...");
                    result.chunk.content = truncated;
                }

            // Remove embeddings if not requested
            if !options.include_embeddings {
                result.chunk.embedding = None;
            }

            // Remove content if not requested
            if !options.include_content {
                result.chunk.content = "[Content hidden]".to_string();
            }
        }

        Ok(results)
    }

    /// Generate context around a chunk
    async fn generate_context(
        &self,
        _chunk: &CodeChunk,
        _context_lines: usize,
    ) -> Option<SearchContext> {
        // TODO: Implement context generation by reading surrounding lines from file
        // For now, return None
        None
    }

    /// Highlight matches in content (simplified implementation)
    fn highlight_matches(&self, content: &str, _query: &str) -> String {
        // TODO: Implement proper highlighting with HTML tags or markdown
        // For now, just return the original content
        content.to_string()
    }

    /// Generate search statistics
    fn generate_stats(&self, results: &[SearchResult]) -> SearchStats {
        let mut match_type_breakdown = HashMap::new();
        let mut language_breakdown = HashMap::new();
        let mut semantic_matches = 0;
        let mut keyword_matches = 0;

        for result in results {
            // Count match types
            *match_type_breakdown
                .entry(result.match_type.clone())
                .or_insert(0) += 1;

            // Count languages
            *language_breakdown
                .entry(result.chunk.language.clone())
                .or_insert(0) += 1;

            // Count specific match types
            match result.match_type {
                MatchType::Semantic => semantic_matches += 1,
                MatchType::ExactKeyword | MatchType::PartialKeyword => keyword_matches += 1,
                MatchType::Hybrid => {
                    semantic_matches += 1;
                    keyword_matches += 1;
                }
                _ => {}
            }
        }

        SearchStats {
            chunks_searched: results.len(), /* This should be total chunks searched, not just
                                             * results */
            semantic_matches,
            keyword_matches,
            filters_applied: 0, // TODO: Count actual filters applied
            match_type_breakdown,
            language_breakdown,
        }
    }

    /// Generate search suggestions
    fn generate_suggestions(&self, query: &SearchQuery, results: &[SearchResult]) -> Vec<String> {
        let mut suggestions = Vec::new();

        // If no results, suggest broader search
        if results.is_empty() {
            suggestions.push("Try using broader search terms".to_string());
            suggestions.push("Check spelling of search terms".to_string());
            suggestions.push("Try using different keywords".to_string());
        }

        // If very few results, suggest related terms
        if results.len() < 3 {
            suggestions.push("Try searching for related terms".to_string());
            if !query.filters.languages.is_empty() {
                suggestions.push("Try removing language filters".to_string());
            }
        }

        // If many results, suggest refinement
        if results.len() > 50 {
            suggestions.push("Try adding more specific terms".to_string());
            suggestions.push("Try filtering by language or path".to_string());
        }

        suggestions
    }
}

// Tests commented out due to lifetime issues - will be implemented in phase 2
