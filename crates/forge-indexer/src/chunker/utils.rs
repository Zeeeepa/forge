//! Utility functions for the chunker

use std::collections::HashSet;

use tree_sitter::{Tree, TreeCursor};

pub(crate) struct ChunkerUtils;

impl ChunkerUtils {
    /// Calculate semantic complexity score for a chunk
    pub fn calculate_semantic_complexity(chunk_code: &str, tree: &Tree, content: &str) -> f32 {
        let mut complexity = 0.0;
        let mut cursor = tree.walk();

        // Traverse the tree and count complexity indicators
        Self::traverse_for_complexity(&mut cursor, content, &mut complexity);

        // Normalize by chunk size
        let lines = chunk_code.lines().count() as f32;
        if lines > 0.0 { complexity / lines } else { 0.0 }
    }

    /// Recursively traverse tree to calculate complexity
    pub fn traverse_for_complexity(cursor: &mut TreeCursor, content: &str, complexity: &mut f32) {
        let node = cursor.node();

        // Add complexity based on node type
        match node.kind() {
            // Control flow structures
            "if_statement" | "if_expression" => *complexity += 1.0,
            "while_statement" | "while_expression" => *complexity += 2.0,
            "for_statement" | "for_expression" => *complexity += 2.0,
            "match_expression" | "switch_statement" => *complexity += 3.0,
            "try_statement" | "try_expression" => *complexity += 2.0,

            // Function definitions (higher complexity)
            "function_item" | "function_definition" | "method_definition" => *complexity += 3.0,
            "closure_expression" | "lambda" => *complexity += 2.0,

            // Class/struct definitions
            "struct_item" | "class_definition" | "impl_item" => *complexity += 4.0,

            // Generic/template usage
            "generic_type" | "type_arguments" => *complexity += 1.5,

            // Async/concurrency
            "async_block" | "await_expression" => *complexity += 2.5,

            _ => {}
        }

        // Traverse children
        if cursor.goto_first_child() {
            loop {
                Self::traverse_for_complexity(cursor, content, complexity);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Detect semantic boundaries in code
    pub fn detect_semantic_boundaries(tree: &Tree, content: &str) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut cursor = tree.walk();

        Self::find_boundaries(&mut cursor, content, &mut boundaries);
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }

    /// Recursively find semantic boundaries
    pub fn find_boundaries(cursor: &mut TreeCursor, content: &str, boundaries: &mut Vec<usize>) {
        let node = cursor.node();

        // Mark boundaries at significant semantic units
        match node.kind() {
            // Top-level definitions
            "function_item"
            | "function_definition"
            | "method_definition"
            | "struct_item"
            | "class_definition"
            | "impl_item"
            | "enum_item"
            | "trait_item"
            | "interface_declaration" => {
                boundaries.push(node.start_byte());
                boundaries.push(node.end_byte());
            }

            // Module/namespace boundaries
            "module" | "mod_item" | "namespace_definition" => {
                boundaries.push(node.start_byte());
                boundaries.push(node.end_byte());
            }

            // Import/use statements (logical groupings)
            "use_declaration" | "import_statement" | "import_from_statement" => {
                boundaries.push(node.end_byte());
            }

            // Comment blocks (documentation boundaries)
            "line_comment" | "block_comment" if node.end_byte() - node.start_byte() > 50 => {
                boundaries.push(node.end_byte());
            }

            _ => {}
        }

        // Traverse children
        if cursor.goto_first_child() {
            loop {
                Self::find_boundaries(cursor, content, boundaries);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Extract keywords from content
    pub fn extract_keywords(content: &str) -> HashSet<String> {
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

    /// Extract symbols (identifiers) from content
    pub fn extract_symbols(content: &str, _lang: &str) -> HashSet<String> {
        let mut symbols = HashSet::new();

        // Fallback: extract camelCase and snake_case identifiers
        for word in content.split_whitespace() {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if Self::is_likely_identifier(clean_word) {
                symbols.insert(clean_word.to_string());
            }
        }

        symbols
    }

    /// Collect symbols from tree-sitter parse tree
    pub fn collect_symbols(cursor: &mut TreeCursor, content: &str, symbols: &mut HashSet<String>) {
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
                Self::collect_symbols(cursor, content, symbols);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Check if a string is likely an identifier
    pub fn is_likely_identifier(word: &str) -> bool {
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

    /// Calculate Jaccard similarity between two sets
    pub fn jaccard_similarity(set1: &HashSet<String>, set2: &HashSet<String>) -> f32 {
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
}
