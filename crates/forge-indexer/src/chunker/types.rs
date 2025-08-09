//! Core data structures for the chunker

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub path: String,
    pub lang: String,
    pub symbol: Option<String>,
    pub rev: String,
    pub size: usize,
    pub code: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CodeSymbol {
    pub name: String,
    pub symbol_type: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub importance_score: f32,
    pub complexity: u32,
    pub references: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct CodeAnalysis {
    pub symbols: Vec<CodeSymbol>,
    pub complexity_score: f32,
    pub semantic_boundaries: Vec<usize>,
    pub imports: Vec<String>,
    pub dependencies: Vec<String>,
}

impl Default for CodeAnalysis {
    fn default() -> Self {
        Self {
            symbols: Vec::new(),
            complexity_score: 0.0,
            semantic_boundaries: Vec::new(),
            imports: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

impl CodeSymbol {
    pub fn calculate_importance(kind: &str) -> f32 {
        match kind {
            "function_item" | "function_definition" | "method_definition" => 1.0,
            "struct_item" | "class_definition" => 1.2,
            "enum_item" | "trait_item" | "interface_declaration" => 1.1,
            "impl_item" => 0.9,
            "const_item" | "static_item" => 0.7,
            "module" => 1.3,
            _ => 0.5,
        }
    }

    pub fn calculate_complexity(node: tree_sitter::Node) -> u32 {
        let mut complexity = 1;

        // Add complexity based on node size and nesting
        complexity += (node.child_count() / 5) as u32;

        // Add complexity for control flow structures
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    "if_statement" | "if_expression" => complexity += 1,
                    "while_statement" | "for_statement" => complexity += 2,
                    "match_expression" | "switch_statement" => complexity += 3,
                    _ => {}
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        complexity
    }
}

impl CodeAnalysis {
    pub fn calculate_overall_complexity(&mut self) {
        if self.symbols.is_empty() {
            self.complexity_score = 0.0;
            return;
        }

        let total_complexity: u32 = self.symbols.iter().map(|s| s.complexity).sum();
        let avg_complexity = total_complexity as f32 / self.symbols.len() as f32;

        // Weight by importance
        let weighted_complexity: f32 = self
            .symbols
            .iter()
            .map(|s| s.complexity as f32 * s.importance_score)
            .sum();

        self.complexity_score = (avg_complexity + weighted_complexity) / 2.0;
    }
}
