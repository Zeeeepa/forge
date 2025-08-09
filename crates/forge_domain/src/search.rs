//! Domain models for search queries and results

use std::collections::HashMap;

use derive_setters::Setters;
use serde::{Deserialize, Serialize};

use crate::code_chunk::CodeChunk;

/// Query structure for semantic search with filters and options
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct SearchQuery {
    /// The search query text
    pub query: String,
    /// Maximum number of results to return
    #[setters(skip)]
    pub limit: usize,
    /// Similarity threshold (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Search mode configuration
    pub mode: SearchMode,
    /// Filters to apply to search results
    pub filters: SearchFilters,
    /// Options for result processing
    pub options: SearchOptions,
}

/// Different search modes available
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchMode {
    /// Pure semantic/vector search
    Semantic,
    /// Traditional keyword/text search
    Keyword,
    /// Hybrid combining semantic and keyword
    Hybrid {
        semantic_weight: f32,
        keyword_weight: f32,
    },
}

/// Filters that can be applied to search results
#[derive(Debug, Clone, Serialize, Deserialize, Setters, Default)]
#[setters(strip_option, into)]
pub struct SearchFilters {
    /// Filter by repository name
    pub repository: Option<String>,
    /// Filter by branch name  
    pub branch: Option<String>,
    /// Filter by programming languages
    pub languages: Vec<String>,
    /// Filter by file paths (glob patterns supported)
    pub paths: Vec<String>,
    /// Filter by symbols/functions
    pub symbols: Vec<String>,
    /// Filter by modification date range
    pub date_range: Option<DateRange>,
    /// Filter by file size range (in bytes)
    pub size_range: Option<SizeRange>,
    /// Filter by complexity score range
    pub complexity_range: Option<ComplexityRange>,
    /// Filter by custom tags
    pub tags: Vec<String>,
    /// Custom metadata filters
    pub metadata: HashMap<String, String>,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start date (inclusive)
    pub start: chrono::DateTime<chrono::Utc>,
    /// End date (inclusive)
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Size range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRange {
    /// Minimum size in bytes
    pub min: u64,
    /// Maximum size in bytes  
    pub max: u64,
}

/// Complexity score range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityRange {
    /// Minimum complexity score
    pub min: f32,
    /// Maximum complexity score
    pub max: f32,
}

/// Options for search result processing
#[derive(Debug, Clone, Serialize, Deserialize, Setters)]
#[setters(strip_option, into)]
pub struct SearchOptions {
    /// Include code content in results
    pub include_content: bool,
    /// Include embeddings in results  
    pub include_embeddings: bool,
    /// Include context around matches
    pub include_context: bool,
    /// Number of context lines to include
    pub context_lines: usize,
    /// Group results by file/symbol
    pub group_by: Option<GroupBy>,
    /// Sort order for results
    pub sort_by: SortBy,
    /// Whether to highlight matches in content
    pub highlight_matches: bool,
    /// Maximum content length to return
    pub max_content_length: Option<usize>,
}

/// Grouping options for search results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroupBy {
    /// Group by file path
    File,
    /// Group by symbol/function
    Symbol,
    /// Group by language
    Language,
    /// Group by repository
    Repository,
}

/// Sorting options for search results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SortBy {
    /// Sort by relevance/similarity score (default)
    Relevance,
    /// Sort by modification date (newest first)
    DateDesc,
    /// Sort by modification date (oldest first)
    DateAsc,
    /// Sort by file path alphabetically
    Path,
    /// Sort by complexity score
    Complexity,
    /// Sort by file size
    Size,
}

/// Search results with metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// The original query
    pub query: SearchQuery,
    /// Matching chunks with scores
    pub chunks: Vec<SearchResult>,
    /// Total number of matches found (before limit)
    pub total_matches: usize,
    /// Time taken to execute search
    pub execution_time_ms: u64,
    /// Search statistics
    pub stats: SearchStats,
    /// Suggestions for query improvement
    pub suggestions: Vec<String>,
}

/// Individual search result with score and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matching code chunk
    pub chunk: CodeChunk,
    /// Similarity/relevance score (0.0 to 1.0)
    pub score: f32,
    /// Type of match found
    pub match_type: MatchType,
    /// Highlighted content with matches
    pub highlighted_content: Option<String>,
    /// Context lines around the match
    pub context: Option<SearchContext>,
    /// Explanation of why this result matched
    pub explanation: Option<String>,
}

/// Type of match found in search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MatchType {
    /// Semantic similarity match
    Semantic,
    /// Exact keyword match
    ExactKeyword,
    /// Partial keyword match
    PartialKeyword,
    /// Symbol/function name match
    Symbol,
    /// Path/filename match
    Path,
    /// Combined hybrid match
    Hybrid,
}

/// Context information around a search match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    /// Lines before the match
    pub before: Vec<String>,
    /// Lines after the match
    pub after: Vec<String>,
    /// File path for context
    pub file_path: String,
    /// Starting line number of context
    pub start_line: usize,
}

/// Statistics about search execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    /// Number of chunks searched
    pub chunks_searched: usize,
    /// Number of semantic matches
    pub semantic_matches: usize,
    /// Number of keyword matches
    pub keyword_matches: usize,
    /// Number of filters applied
    pub filters_applied: usize,
    /// Breakdown by match type
    pub match_type_breakdown: HashMap<MatchType, usize>,
    /// Breakdown by language
    pub language_breakdown: HashMap<String, usize>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 20,
            similarity_threshold: 0.7,
            mode: SearchMode::Hybrid { semantic_weight: 0.7, keyword_weight: 0.3 },
            filters: SearchFilters::default(),
            options: SearchOptions::default(),
        }
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            include_content: true,
            include_embeddings: false,
            include_context: true,
            context_lines: 3,
            group_by: None,
            sort_by: SortBy::Relevance,
            highlight_matches: true,
            max_content_length: Some(2000),
        }
    }
}

impl SearchQuery {
    /// Create a simple semantic search query
    pub fn semantic(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: SearchMode::Semantic,
            ..Default::default()
        }
    }

    /// Create a simple keyword search query  
    pub fn keyword(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: SearchMode::Keyword,
            ..Default::default()
        }
    }

    /// Create a hybrid search query with custom weights
    pub fn hybrid(query: impl Into<String>, semantic_weight: f32, keyword_weight: f32) -> Self {
        Self {
            query: query.into(),
            mode: SearchMode::Hybrid { semantic_weight, keyword_weight },
            ..Default::default()
        }
    }

    /// Add a repository filter
    pub fn repository(mut self, repo: impl Into<String>) -> Self {
        self.filters.repository = Some(repo.into());
        self
    }

    /// Add language filters
    pub fn languages(mut self, langs: Vec<String>) -> Self {
        self.filters.languages = langs;
        self
    }

    /// Set result limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set similarity threshold
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

impl SearchResults {
    /// Check if search found any results
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get the number of results returned
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Get results grouped by file path
    pub fn group_by_file(&self) -> HashMap<String, Vec<&SearchResult>> {
        let mut groups = HashMap::new();
        for result in &self.chunks {
            groups
                .entry(result.chunk.path.clone())
                .or_insert_with(Vec::new)
                .push(result);
        }
        groups
    }

    /// Get the top N results
    pub fn top(&self, n: usize) -> &[SearchResult] {
        &self.chunks[..n.min(self.chunks.len())]
    }

    /// Filter results by minimum score
    pub fn filter_by_score(&self, min_score: f32) -> Vec<&SearchResult> {
        self.chunks
            .iter()
            .filter(|result| result.score >= min_score)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_search_query_creation() {
        let fixture = SearchQuery::semantic("test query");

        let actual = fixture.mode;
        let expected = SearchMode::Semantic;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_search_query_builder() {
        let fixture = SearchQuery::hybrid("rust function", 0.8, 0.2)
            .repository("my-repo")
            .languages(vec!["rust".to_string()])
            .limit(10)
            .threshold(0.8);

        let actual_repo = fixture.filters.repository.clone();
        let actual_langs = fixture.filters.languages.clone();
        let actual_limit = fixture.limit;
        let actual_threshold = fixture.similarity_threshold;

        let expected_repo = Some("my-repo".to_string());
        let expected_langs = vec!["rust".to_string()];
        let expected_limit = 10;
        let expected_threshold = 0.8;

        assert_eq!(actual_repo, expected_repo);
        assert_eq!(actual_langs, expected_langs);
        assert_eq!(actual_limit, expected_limit);
        assert_eq!(actual_threshold, expected_threshold);
    }

    #[test]
    fn test_search_results_is_empty() {
        let fixture = SearchResults {
            query: SearchQuery::default(),
            chunks: vec![],
            total_matches: 0,
            execution_time_ms: 0,
            stats: SearchStats {
                chunks_searched: 0,
                semantic_matches: 0,
                keyword_matches: 0,
                filters_applied: 0,
                match_type_breakdown: HashMap::new(),
                language_breakdown: HashMap::new(),
            },
            suggestions: vec![],
        };

        let actual = fixture.is_empty();
        let expected = true;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_threshold_clamping() {
        let fixture = SearchQuery::semantic("test").threshold(1.5);

        let actual = fixture.similarity_threshold;
        let expected = 1.0;

        assert_eq!(actual, expected);
    }
}
