//! Different chunking strategies

use tree_sitter::{Tree, TreeCursor};

use crate::chunker::analysis::CodeAnalyzer;
use crate::chunker::types::{Chunk, CodeAnalysis};

pub(crate) struct ChunkingStrategies;

impl ChunkingStrategies {
    /// Extract semantic chunks based on code structure
    pub fn extract_semantic_chunks(
        path: &str,
        content: &str,
        lang: &str,
        rev: &str,
        analysis: &CodeAnalysis,
        tree: &Tree,
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut cursor = tree.walk();

        // Chunk by top-level definitions (functions, classes, etc.)
        Self::chunk_by_definitions(&mut cursor, content, path, lang, rev, &mut chunks);

        // If chunks are too large, subdivide them
        let mut refined_chunks = Vec::new();
        for chunk in chunks {
            if chunk.size > 2000 {
                refined_chunks.extend(Self::subdivide_large_chunk(&chunk, analysis, tree));
            } else {
                refined_chunks.push(chunk);
            }
        }

        refined_chunks
    }

    /// Extract context-aware chunks using comprehensive semantic analysis
    pub fn extract_context_chunks(
        path: &str,
        content: &str,
        lang: &str,
        rev: &str,
        analysis: &CodeAnalysis,
        tree: &Tree,
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();

        // If no semantic analysis available, fallback to basic chunking
        if analysis.symbols.is_empty() && analysis.semantic_boundaries.is_empty() {
            return Self::fallback_chunking(path, content, lang, rev, Some(tree));
        }

        // Strategy 1: Symbol-based chunking for high-importance symbols
        let high_importance_symbols: Vec<_> = analysis
            .symbols
            .iter()
            .filter(|s| s.importance_score > 0.7 || s.references.len() > 2)
            .collect();

        if !high_importance_symbols.is_empty() {
            for symbol in high_importance_symbols {
                let start = symbol.start_byte;
                let end = symbol.end_byte.min(content.len());

                if end > start && end - start >= 50 {
                    let chunk_content = &content[start..end];

                    // Create enhanced summary with context
                    let mut summary_parts =
                        vec![format!("{}: {}", symbol.symbol_type, symbol.name)];

                    if symbol.complexity > 5 {
                        summary_parts.push(format!("High complexity: {}", symbol.complexity));
                    }

                    if !symbol.references.is_empty() {
                        summary_parts.push(format!("Referenced {} times", symbol.references.len()));
                    }

                    // Add import context if this chunk contains imports
                    let chunk_imports: Vec<_> = analysis
                        .imports
                        .iter()
                        .filter(|import| chunk_content.contains(import.as_str()))
                        .cloned()
                        .collect();

                    if !chunk_imports.is_empty() {
                        summary_parts.push(format!("Imports: {}", chunk_imports.join(", ")));
                    }

                    chunks.push(Chunk {
                        id: format!("{path}:symbol:{start}:{end}"),
                        path: path.to_string(),
                        lang: lang.to_string(),
                        symbol: Some(symbol.name.clone()),
                        rev: rev.to_string(),
                        size: end - start,
                        code: chunk_content.to_string(),
                        summary: Some(summary_parts.join(" | ")),
                    });
                }
            }
        }

        // Strategy 2: Semantic boundary-based chunking for remaining content
        let mut covered_ranges = chunks
            .iter()
            .map(|c| {
                let parts: Vec<&str> = c.id.split(':').collect();
                if parts.len() >= 4 && parts[1] == "symbol" {
                    let start = parts[2].parse::<usize>().unwrap_or(0);
                    let end = parts[3].parse::<usize>().unwrap_or(0);
                    (start, end)
                } else {
                    (0, 0)
                }
            })
            .collect::<Vec<_>>();

        covered_ranges.sort_by_key(|&(start, _)| start);

        let boundaries = &analysis.semantic_boundaries;
        if !boundaries.is_empty() {
            let mut start = 0;

            for &boundary in boundaries {
                // Skip if this range is already covered by symbol-based chunks
                let is_covered = covered_ranges
                    .iter()
                    .any(|&(cstart, cend)| start >= cstart && boundary <= cend);

                if !is_covered && boundary > start + 100 {
                    let chunk_content = &content[start..boundary];
                    let primary_symbol =
                        CodeAnalyzer::extract_primary_symbol(chunk_content, lang, tree);

                    // Create contextual summary
                    let mut summary_parts = Vec::new();

                    // Add complexity context
                    if analysis.complexity_score > 0.5 {
                        summary_parts.push("Complex code section".to_string());
                    }

                    // Add dependency context
                    let chunk_deps: Vec<_> = analysis
                        .dependencies
                        .iter()
                        .filter(|dep| chunk_content.contains(dep.as_str()))
                        .cloned()
                        .collect();

                    if !chunk_deps.is_empty() {
                        summary_parts.push(format!("Dependencies: {}", chunk_deps.join(", ")));
                    }

                    // Find related symbols in this chunk
                    let related_symbols: Vec<_> = analysis
                        .symbols
                        .iter()
                        .filter(|s| s.start_byte >= start && s.end_byte <= boundary)
                        .map(|s| s.name.clone())
                        .collect();

                    if !related_symbols.is_empty() {
                        summary_parts.push(format!("Contains: {}", related_symbols.join(", ")));
                    }

                    chunks.push(Chunk {
                        id: format!("{path}:semantic:{start}:{boundary}"),
                        path: path.to_string(),
                        lang: lang.to_string(),
                        symbol: primary_symbol,
                        rev: rev.to_string(),
                        size: boundary - start,
                        code: chunk_content.to_string(),
                        summary: if summary_parts.is_empty() {
                            None
                        } else {
                            Some(summary_parts.join(" | "))
                        },
                    });

                    start = boundary;
                }
            }

            // Handle remaining content
            if start < content.len() {
                let is_covered = covered_ranges
                    .iter()
                    .any(|&(cstart, cend)| start >= cstart && content.len() <= cend);

                if !is_covered {
                    let chunk_content = &content[start..];
                    let primary_symbol =
                        CodeAnalyzer::extract_primary_symbol(chunk_content, lang, tree);

                    // Add final chunk context
                    let mut summary_parts = vec!["Final section".to_string()];

                    let remaining_symbols: Vec<_> = analysis
                        .symbols
                        .iter()
                        .filter(|s| s.start_byte >= start)
                        .map(|s| s.name.clone())
                        .collect();

                    if !remaining_symbols.is_empty() {
                        summary_parts.push(format!("Contains: {}", remaining_symbols.join(", ")));
                    }

                    chunks.push(Chunk {
                        id: format!("{}:final:{}:{}", path, start, content.len()),
                        path: path.to_string(),
                        lang: lang.to_string(),
                        symbol: primary_symbol,
                        rev: rev.to_string(),
                        size: content.len() - start,
                        code: chunk_content.to_string(),
                        summary: Some(summary_parts.join(" | ")),
                    });
                }
            }
        }

        // If no chunks were created, fallback to basic chunking
        if chunks.is_empty() {
            return Self::fallback_chunking(path, content, lang, rev, Some(tree));
        }

        // Sort chunks by start position for consistent ordering
        chunks.sort_by_key(|chunk| {
            let parts: Vec<&str> = chunk.id.split(':').collect();
            if parts.len() >= 3 {
                parts[parts.len() - 2].parse::<usize>().unwrap_or(0)
            } else {
                0
            }
        });

        chunks
    }

    /// Fallback chunking for unsupported languages
    pub fn fallback_chunking(
        path: &str,
        content: &str,
        lang: &str,
        rev: &str,
        _tree: Option<&Tree>,
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut start_line = 0;

        while start_line < lines.len() {
            let mut end_line = (start_line + 50).min(lines.len());

            // Try to break at natural boundaries (empty lines, comments)
            if end_line < lines.len() {
                for i in (start_line + 30..end_line).rev() {
                    if lines[i].trim().is_empty()
                        || lines[i].trim_start().starts_with("//")
                        || lines[i].trim_start().starts_with('#')
                    {
                        end_line = i + 1;
                        break;
                    }
                }
            }

            let chunk_lines = &lines[start_line..end_line];
            let chunk_content = chunk_lines.join("\n");

            chunks.push(Chunk {
                id: format!("{path}:{start_line}:{end_line}"),
                path: path.to_string(),
                lang: lang.to_string(),
                symbol: None,
                rev: rev.to_string(),
                size: chunk_content.len(),
                code: chunk_content,
                summary: None,
            });

            start_line = end_line;
        }

        chunks
    }

    fn chunk_by_definitions(
        cursor: &mut TreeCursor,
        content: &str,
        path: &str,
        lang: &str,
        rev: &str,
        chunks: &mut Vec<Chunk>,
    ) {
        let node = cursor.node();

        if Self::is_symbol_node(node.kind(), lang) {
            let chunk_content = &content[node.byte_range()];
            let symbol = CodeAnalyzer::extract_symbol_name(node, content, lang);

            chunks.push(Chunk {
                id: format!("{}:{}:{}", path, node.start_byte(), node.end_byte()),
                path: path.to_string(),
                lang: lang.to_string(),
                symbol,
                rev: rev.to_string(),
                size: chunk_content.len(),
                code: chunk_content.to_string(),
                summary: None,
            });
        }

        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                Self::chunk_by_definitions(cursor, content, path, lang, rev, chunks);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn subdivide_large_chunk(chunk: &Chunk, _analysis: &CodeAnalysis, _tree: &Tree) -> Vec<Chunk> {
        // For now, simple subdivision by lines
        // TODO: Implement more sophisticated subdivision based on semantic analysis
        let lines: Vec<&str> = chunk.code.lines().collect();
        let mut sub_chunks = Vec::new();
        let chunk_size = 50; // lines per sub-chunk

        for (i, lines_chunk) in lines.chunks(chunk_size).enumerate() {
            let sub_content = lines_chunk.join("\n");
            sub_chunks.push(Chunk {
                id: format!("{}.{}", chunk.id, i),
                path: chunk.path.clone(),
                lang: chunk.lang.clone(),
                symbol: chunk.symbol.clone(),
                rev: chunk.rev.clone(),
                size: sub_content.len(),
                code: sub_content,
                summary: None,
            });
        }

        sub_chunks
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
}
