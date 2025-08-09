//! Search services and utilities

mod search_service;

pub use search_service::SearchService;

// Re-export utility functions
pub use utils::{
    quick_semantic_search, quick_keyword_search, quick_hybrid_search,
    extract_function_names, calculate_relevance_score
};

/// Factory for creating search service instances
pub struct SearchServiceFactory;

impl SearchServiceFactory {
    /// Create a search service with the given vector store and embedder
    pub fn create(
        vector_store: crate::vector_store::SharedVectorStore,
        embedder: Box<dyn crate::indexing::Embedder>,
    ) -> SearchService {
        SearchService::new(vector_store, embedder)
    }
}

/// Utility functions for search operations
pub mod utils {
    use forge_domain::{SearchMode, SearchOptions, SearchQuery, SortBy};

    /// Create a quick semantic search query
    pub fn quick_semantic_search(query: impl Into<String>, limit: usize) -> SearchQuery {
        SearchQuery {
            query: query.into(),
            limit,
            similarity_threshold: 0.7,
            mode: SearchMode::Semantic,
            filters: Default::default(),
            options: SearchOptions {
                include_content: true,
                include_embeddings: false,
                include_context: false,
                context_lines: 0,
                group_by: None,
                sort_by: SortBy::Relevance,
                highlight_matches: false,
                max_content_length: Some(500),
            },
        }
    }

    /// Create a quick keyword search query
    pub fn quick_keyword_search(query: impl Into<String>, limit: usize) -> SearchQuery {
        SearchQuery {
            query: query.into(),
            limit,
            similarity_threshold: 0.0, // Not used for keyword search
            mode: SearchMode::Keyword,
            filters: Default::default(),
            options: SearchOptions {
                include_content: true,
                include_embeddings: false,
                include_context: true,
                context_lines: 2,
                group_by: None,
                sort_by: SortBy::Relevance,
                highlight_matches: true,
                max_content_length: Some(1000),
            },
        }
    }

    /// Create a balanced hybrid search query
    pub fn quick_hybrid_search(query: impl Into<String>, limit: usize) -> SearchQuery {
        SearchQuery {
            query: query.into(),
            limit,
            similarity_threshold: 0.6,
            mode: SearchMode::Hybrid { semantic_weight: 0.7, keyword_weight: 0.3 },
            filters: Default::default(),
            options: SearchOptions {
                include_content: true,
                include_embeddings: false,
                include_context: true,
                context_lines: 3,
                group_by: None,
                sort_by: SortBy::Relevance,
                highlight_matches: true,
                max_content_length: Some(800),
            },
        }
    }

    /// Extract function names from code content
    pub fn extract_function_names(content: &str, language: &str) -> Vec<String> {
        match language {
            "rust" => extract_rust_functions(content),
            "python" => extract_python_functions(content),
            "javascript" | "typescript" => extract_js_functions(content),
            "java" => extract_java_functions(content),
            _ => Vec::new(),
        }
    }

    /// Extract Rust function names
    fn extract_rust_functions(content: &str) -> Vec<String> {
        let mut functions = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("fn ") || trimmed.starts_with("pub fn "))
                && let Some(name) = extract_function_name_from_signature(trimmed, "fn ") {
                    functions.push(name);
                }
        }
        functions
    }

    /// Extract Python function names
    fn extract_python_functions(content: &str) -> Vec<String> {
        let mut functions = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("def ")
                && let Some(name) = extract_function_name_from_signature(trimmed, "def ") {
                    functions.push(name);
                }
        }
        functions
    }

    /// Extract JavaScript/TypeScript function names
    fn extract_js_functions(content: &str) -> Vec<String> {
        let mut functions = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("function ") {
                if let Some(name) = extract_function_name_from_signature(trimmed, "function ") {
                    functions.push(name);
                }
            } else if trimmed.contains(" = function") || trimmed.contains(" => ") {
                // Arrow functions and function expressions
                if let Some(start) = trimmed
                    .find("const ")
                    .or_else(|| trimmed.find("let ").or_else(|| trimmed.find("var ")))
                    && let Some(end) = trimmed[start..].find(' ') {
                        let name = &trimmed[start..start + end];
                        if !name.is_empty() {
                            functions.push(name.to_string());
                        }
                    }
            }
        }
        functions
    }

    /// Extract Java function names
    fn extract_java_functions(content: &str) -> Vec<String> {
        let mut functions = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            // Look for method signatures (simplified)
            if (trimmed.contains("public ")
                || trimmed.contains("private ")
                || trimmed.contains("protected "))
                && trimmed.contains('(')
                && trimmed.contains('{')
            {
                // Extract method name (very simplified)
                if let Some(paren_pos) = trimmed.find('(') {
                    let before_paren = &trimmed[..paren_pos];
                    if let Some(last_space) = before_paren.rfind(' ') {
                        let name = &before_paren[last_space + 1..];
                        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                        {
                            functions.push(name.to_string());
                        }
                    }
                }
            }
        }
        functions
    }

    /// Extract function name from a function signature
    fn extract_function_name_from_signature(signature: &str, prefix: &str) -> Option<String> {
        if let Some(start) = signature.find(prefix) {
            let after_prefix = &signature[start + prefix.len()..];
            if let Some(end) = after_prefix.find('(').or_else(|| after_prefix.find(' ')) {
                let name = &after_prefix[..end];
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    /// Calculate search relevance score combining multiple factors
    pub fn calculate_relevance_score(
        semantic_score: f32,
        keyword_score: f32,
        path_match: bool,
        symbol_match: bool,
    ) -> f32 {
        let mut score = semantic_score * 0.4 + keyword_score * 0.3;

        if path_match {
            score += 0.15;
        }

        if symbol_match {
            score += 0.15;
        }

        score.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use forge_domain::SearchMode;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_extract_rust_functions() {
        let content = r#"
            fn main() {
                println!("Hello");
            }
            
            pub fn test_function() -> i32 {
                42
            }
        "#;

        let actual = utils::extract_function_names(content, "rust");
        let expected = vec!["main".to_string(), "test_function".to_string()];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_extract_python_functions() {
        let content = r#"
            def main():
                print("Hello")
            
            def test_function(x, y):
                return x + y
        "#;

        let actual = utils::extract_function_names(content, "python");
        let expected = vec!["main".to_string(), "test_function".to_string()];

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_quick_search_builders() {
        let semantic_query = utils::quick_semantic_search("test", 10);
        let keyword_query = utils::quick_keyword_search("test", 10);
        let hybrid_query = utils::quick_hybrid_search("test", 10);

        assert!(matches!(semantic_query.mode, SearchMode::Semantic));
        assert!(matches!(keyword_query.mode, SearchMode::Keyword));
        assert!(matches!(hybrid_query.mode, SearchMode::Hybrid { .. }));
    }

    #[test]
    fn test_calculate_relevance_score() {
        let fixtures = vec![
            (0.8, 0.6, true, true, 0.8),    // High scores with matches
            (0.5, 0.3, false, false, 0.29), // Medium scores without matches
            (1.0, 1.0, true, true, 1.0),    // Max score should be capped at 1.0
        ];

        for (semantic, keyword, path, symbol, expected) in fixtures {
            let actual = utils::calculate_relevance_score(semantic, keyword, path, symbol);
            assert!(
                (actual - expected).abs() < 0.01,
                "Expected {}, got {} for inputs: {}, {}, {}, {}",
                expected,
                actual,
                semantic,
                keyword,
                path,
                symbol
            );
        }
    }
}
