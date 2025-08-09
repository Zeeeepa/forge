//! Code analysis and AST processing functions

use tree_sitter::{Tree, TreeCursor};

use crate::chunker::types::{CodeAnalysis, CodeSymbol};

pub(crate) struct CodeAnalyzer;

impl CodeAnalyzer {
    /// Analyze code structure using AST
    pub fn analyze_code_structure(content: &str, lang: &str, tree: &Tree) -> CodeAnalysis {
        let mut analysis = CodeAnalysis::default();
        let mut cursor = tree.walk();

        // Extract symbols and calculate complexity using all available methods
        Self::extract_symbols_recursive(&mut cursor, content, &mut analysis.symbols, lang);

        // Calculate overall complexity using symbol analysis
        analysis.calculate_overall_complexity();

        // Find semantic boundaries using multiple methods for comprehensive coverage
        let mut boundaries = Self::find_semantic_boundaries_enhanced(tree, content);
        let additional_boundaries = Self::find_semantic_boundaries(tree, content);
        boundaries.extend(additional_boundaries);
        boundaries.sort_unstable();
        boundaries.dedup();
        analysis.semantic_boundaries = boundaries;

        // Extract imports and dependencies using enhanced methods
        cursor = tree.walk(); // Reset cursor
        Self::extract_imports(&mut cursor, content, &mut analysis.imports, lang);

        // Extract dependencies from imports and other analysis
        analysis.dependencies = analysis.imports.clone();

        // Add additional dependencies from symbol analysis
        let symbol_deps: Vec<String> = analysis
            .symbols
            .iter()
            .filter_map(|symbol| {
                if symbol.symbol_type == "import" || symbol.symbol_type == "use" {
                    Some(symbol.name.clone())
                } else {
                    None
                }
            })
            .collect();
        analysis.dependencies.extend(symbol_deps);
        analysis.dependencies.sort();
        analysis.dependencies.dedup();

        analysis
    }

    /// Enhanced semantic boundary detection using the new implementation
    pub fn find_semantic_boundaries_enhanced(tree: &Tree, content: &str) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut cursor = tree.walk();

        Self::find_boundaries_recursive(&mut cursor, content, &mut boundaries);

        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }

    pub fn find_semantic_boundaries(tree: &Tree, content: &str) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut cursor = tree.walk();

        Self::find_boundaries_recursive(&mut cursor, content, &mut boundaries);

        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }

    fn extract_symbols_recursive(
        cursor: &mut TreeCursor,
        content: &str,
        symbols: &mut Vec<CodeSymbol>,
        lang: &str,
    ) {
        let node = cursor.node();

        // Check if this node represents a symbol we care about
        if Self::is_symbol_node(node.kind(), lang)
            && let Some(symbol_name) = Self::extract_symbol_name(node, content, lang)
        {
            symbols.push(CodeSymbol {
                name: symbol_name,
                symbol_type: node.kind().to_string(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                importance_score: CodeSymbol::calculate_importance(node.kind()),
                complexity: CodeSymbol::calculate_complexity(node),
                references: Vec::new(),
            });
        }

        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                Self::extract_symbols_recursive(cursor, content, symbols, lang);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn is_symbol_node(kind: &str, _lang: &str) -> bool {
        matches!(
            kind,
            "function_item"
                | "function_definition"
                | "method_definition"
                | "struct_item"
                | "class_definition"
                | "enum_item"
                | "trait_item"
                | "impl_item"
                | "type_item"
                | "const_item"
                | "static_item"
                | "module"
                | "interface_declaration"
        )
    }

    fn find_boundaries_recursive(
        cursor: &mut TreeCursor,
        _content: &str,
        boundaries: &mut Vec<usize>,
    ) {
        let node = cursor.node();

        // Mark boundaries at significant semantic units
        match node.kind() {
            "function_item"
            | "function_definition"
            | "method_definition"
            | "struct_item"
            | "class_definition"
            | "impl_item"
            | "enum_item"
            | "trait_item"
            | "interface_declaration"
            | "module"
            | "mod_item"
            | "namespace_definition" => {
                boundaries.push(node.start_byte());
                boundaries.push(node.end_byte());
            }
            "use_declaration" | "import_statement" | "import_from_statement" => {
                boundaries.push(node.end_byte());
            }
            "line_comment" | "block_comment" if node.end_byte() - node.start_byte() > 50 => {
                boundaries.push(node.end_byte());
            }
            _ => {}
        }

        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                Self::find_boundaries_recursive(cursor, _content, boundaries);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn extract_imports(
        cursor: &mut TreeCursor,
        content: &str,
        imports: &mut Vec<String>,
        lang: &str,
    ) {
        let node = cursor.node();

        let is_import = match lang {
            "rust" => node.kind() == "use_declaration",
            "python" => node.kind() == "import_statement" || node.kind() == "import_from_statement",
            "javascript" | "typescript" => {
                node.kind() == "import_statement" || node.kind() == "import_declaration"
            }
            "go" => node.kind() == "import_declaration",
            "java" => node.kind() == "import_declaration",
            _ => false,
        };

        if is_import {
            let import_text = &content[node.byte_range()];
            imports.push(import_text.to_string());
        }

        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                Self::extract_imports(cursor, content, imports, lang);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Extract symbol name from a node
    pub fn extract_symbol_name(
        node: tree_sitter::Node,
        content: &str,
        lang: &str,
    ) -> Option<String> {
        let mut cursor = node.walk();

        match lang {
            "rust" => {
                // For rust, look for identifiers in function_item, struct_item, etc.
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "identifier" {
                            return Some(content[child.byte_range()].to_string());
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                None
            }
            "python" => {
                // For python, look for identifiers in function_definition, class_definition,
                // etc.
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "identifier" {
                            return Some(content[child.byte_range()].to_string());
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                None
            }
            "javascript" | "typescript" => {
                // For JS/TS, look for identifiers in function declarations, class declarations,
                // etc.
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "identifier" {
                            return Some(content[child.byte_range()].to_string());
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                None
            }
            "go" => {
                // For Go, look for identifiers in function declarations, type declarations,
                // etc.
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "identifier" {
                            return Some(content[child.byte_range()].to_string());
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                None
            }
            "java" => {
                // For Java, look for identifiers in method declarations, class declarations,
                // etc.
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "identifier" {
                            return Some(content[child.byte_range()].to_string());
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                None
            }
            "cpp" | "c" => {
                // For C/C++, look for identifiers in function declarations, class/struct
                // declarations, etc.
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "identifier" {
                            return Some(content[child.byte_range()].to_string());
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn extract_primary_symbol(content: &str, lang: &str, tree: &Tree) -> Option<String> {
        let mut cursor = tree.walk();
        Self::find_primary_symbol(&mut cursor, content, lang)
    }

    fn find_primary_symbol(cursor: &mut TreeCursor, content: &str, lang: &str) -> Option<String> {
        let node = cursor.node();

        if Self::is_symbol_node(node.kind(), lang)
            && let Some(symbol) = Self::extract_symbol_name(node, content, lang)
        {
            return Some(symbol);
        }

        // Recursively search children
        if cursor.goto_first_child() {
            loop {
                if let Some(symbol) = Self::find_primary_symbol(cursor, content, lang) {
                    return Some(symbol);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        None
    }
}
