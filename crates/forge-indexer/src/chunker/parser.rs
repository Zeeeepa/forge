//! Parser management and initialization

use std::collections::HashMap;

use tree_sitter::Parser;

pub(crate) struct ParserManager {
    parsers: HashMap<String, Parser>,
}

impl Default for ParserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserManager {
    pub fn new() -> Self {
        let mut parsers = HashMap::new();

        // Initialize parsers for supported languages
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert("rust".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert("python".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .is_ok()
        {
            parsers.insert("javascript".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .is_ok()
        {
            parsers.insert("typescript".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert("go".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert("java".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert("cpp".to_string(), parser);
        }

        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .is_ok()
        {
            parsers.insert("c".to_string(), parser);
        }

        Self { parsers }
    }

    pub fn get_parser(&mut self, lang: &str) -> Option<&mut Parser> {
        self.parsers.get_mut(lang)
    }

    pub fn has_parser(&self, lang: &str) -> bool {
        self.parsers.contains_key(lang)
    }

    /// Get appropriate parser for language (creates a new instance)
    pub fn create_parser(&self, lang: &str) -> Option<Parser> {
        if !self.parsers.contains_key(lang) {
            return None;
        }

        let mut new_parser = Parser::new();

        // Set the language based on the lang parameter
        match lang {
            "rust" => new_parser
                .set_language(&tree_sitter_rust::LANGUAGE.into())
                .ok()?,
            "python" => new_parser
                .set_language(&tree_sitter_python::LANGUAGE.into())
                .ok()?,
            "javascript" => new_parser
                .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                .ok()?,
            "typescript" => new_parser
                .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                .ok()?,
            "go" => new_parser
                .set_language(&tree_sitter_go::LANGUAGE.into())
                .ok()?,
            "java" => new_parser
                .set_language(&tree_sitter_java::LANGUAGE.into())
                .ok()?,
            "cpp" | "c" => new_parser
                .set_language(&tree_sitter_cpp::LANGUAGE.into())
                .ok()?,
            _ => return None,
        }

        Some(new_parser)
    }
}
