use std::collections::HashMap;

use crate::utils::tree_sitter_helpers::dfs_visit;
use line_index::{LineIndex, TextSize, WideLineCol};
use tower_lsp::lsp_types::Position;
use tree_sitter::{Parser, Tree};

#[derive(Debug)]
pub struct Doc {
    pub text: String,
    pub version: i32,
    pub tree: Tree,
    pub index: Index,
    pub line_index: LineIndex,
}

impl Doc {
    pub fn new(text: String, version: i32, tree: Tree) -> Self {
        let line_index = LineIndex::new(&text);
        let index = Index::build(&tree, &text);
        Self {
            text,
            version,
            tree,
            index,
            line_index,
        }
    }

    pub fn update(&mut self, text: String, version: i32, parser: &mut Parser) {
        self.text = text;
        self.version = version;
        self.tree = parser
            .parse(&self.text, None)
            .expect("parse() returned None");
        self.index = Index::build(&self.tree, &self.text);
        self.line_index = LineIndex::new(&self.text);
    }

    // Convert LSP position (UTF-16) to byte offset
    pub fn lsp_position_to_offset(&self, position: Position) -> Option<usize> {
        let wide_line_col = WideLineCol {
            line: position.line,
            col: position.character,
        };
        self.line_index
            .to_utf8(line_index::WideEncoding::Utf16, wide_line_col)
            .and_then(|line_col| self.line_index.offset(line_col))
            .map(|text_size| text_size.into()) // Convert TextSize to usize
    }

    // Convert byte offset to LSP position (UTF-16)
    pub fn offset_to_lsp_position(&self, offset: usize) -> Option<Position> {
        let text_size = TextSize::try_from(offset as u32).ok()?;
        let line_col = self.line_index.line_col(text_size);
        let wide_line_col = self
            .line_index
            .to_wide(line_index::WideEncoding::Utf16, line_col)?;

        Some(Position {
            line: wide_line_col.line,
            character: wide_line_col.col,
        })
    }
}

pub fn make_parser() -> Parser {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_froggy::LANGUAGE.into())
        .expect("load language");
    p
}

#[derive(Default, Clone, Debug)]
pub struct Index {
    pub label_defs: HashMap<String, ByteRange>,
    pub label_refs: HashMap<String, Vec<ByteRange>>,
}

impl Index {
    pub fn build(tree: &Tree, text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut idx = Index::default();

        dfs_visit(tree, |node| match node.kind() {
            "label_definition" => {
                let id = node.child_by_field_name("name").or_else(|| node.child(1));
                if let Some(id) = id {
                    if let Ok(name) = id.utf8_text(bytes) {
                        idx.label_defs.insert(
                            name.to_string(),
                            ByteRange {
                                start: node.start_byte(),
                                end: node.end_byte(),
                            },
                        );
                    }
                }
            }
            "label_reference" => {
                let id = node.child_by_field_name("name").or_else(|| node.child(0));
                if let Some(id) = id {
                    if let Ok(name) = id.utf8_text(bytes) {
                        idx.label_refs
                            .entry(name.to_string())
                            .or_default()
                            .push(ByteRange {
                                start: id.start_byte(),
                                end: id.end_byte(),
                            });
                    }
                }
            }
            _ => {}
        });

        idx
    }
}

#[derive(Clone, Debug)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}
