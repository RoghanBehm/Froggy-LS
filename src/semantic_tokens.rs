use tower_lsp::lsp_types::SemanticTokens;
use tower_lsp::lsp_types::*;

use crate::document::{ByteRange, Doc};
use crate::utils::tree_sitter_helpers::dfs_visit;

pub struct Tok {
    line: u32,
    col: u32,
    len: u32,
    ty: u32,   // index into legend.token_types
    mods: u32, // bitset index into legend.token_modifiers
}

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::NUMBER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::VARIABLE,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
        ],
    }
}

// token type indices
pub mod token_types {
    pub const KEYWORD: u32 = 0;
    pub const NUMBER: u32 = 1;
    pub const COMMENT: u32 = 2;
    pub const STRING: u32 = 3;
    pub const VARIABLE: u32 = 4;
}

// modifier bitset flags
pub mod token_modifiers {
    pub const DECLARATION: u32 = 1 << 0; // index 0
    pub const DEFINITION: u32 = 1 << 1; // index 1
    pub const READONLY: u32 = 1 << 2; // index 2
}

pub fn build_semantic_tokens(doc: &Doc) -> Vec<Tok> {
    let mut toks = Vec::new();

    dfs_visit(&doc.tree, |node| {
        let kind = node.kind();
        eprintln!("=== Building semantic tokens ===");
        eprintln!("Tree root: {}", doc.tree.root_node().kind());

        // Map tree-sitter node kinds to semantic token types
        let (token_type, modifiers) = match kind {
            "plop" | "hop" | "leap" | "ribbit" | "croak" => (Some(token_types::KEYWORD), 0),

            "number" | "integer" | "float" => (Some(token_types::NUMBER), 0),

            "comment" | "line_comment" | "block_comment" => (Some(token_types::COMMENT), 0),

            "string" | "string_literal" => (Some(token_types::STRING), 0),

            "label_definition" => {
                if let Some(name_node) = node.child_by_field_name("name").or_else(|| node.child(1))
                {
                    let range = ByteRange {
                        start: name_node.start_byte(),
                        end: name_node.end_byte(),
                    };
                    if let Some(t) = tok_from_range(
                        doc,
                        &range,
                        token_types::VARIABLE,
                        token_modifiers::DECLARATION | token_modifiers::DEFINITION,
                    ) {
                        toks.push(t);
                    }
                }
                return;
            }

            "label_reference" => {
                if let Some(name_node) = node.child_by_field_name("name").or_else(|| node.child(0))
                {
                    let range = ByteRange {
                        start: name_node.start_byte(),
                        end: name_node.end_byte(),
                    };
                    if let Some(t) = tok_from_range(doc, &range, token_types::VARIABLE, 0) {
                        toks.push(t);
                    }
                }
                return;
            }

            "identifier" => {
                if let Some(parent) = node.parent() {
                    if parent.kind() == "label_definition" || parent.kind() == "label_reference" {
                        return;
                    }
                }
                (Some(token_types::VARIABLE), 0)
            }

            _ => (None, 0),
        };

        if let Some(ty) = token_type {
            let range = ByteRange {
                start: node.start_byte(),
                end: node.end_byte(),
            };
            if let Some(t) = tok_from_range(doc, &range, ty, modifiers) {
                toks.push(t);
            }
        }
    });

    toks
}

fn tok_from_range(doc: &Doc, r: &ByteRange, ty: u32, mods: u32) -> Option<Tok> {
    let start = doc.offset_to_lsp_position(r.start)?;
    let end = doc.offset_to_lsp_position(r.end)?;
    if start.line != end.line {
        return None;
    }

    Some(Tok {
        line: start.line,
        col: start.character,
        len: end.character - start.character,
        ty,
        mods,
    })
}

pub fn encode_semantic_tokens(mut toks: Vec<Tok>) -> SemanticTokens {
    toks.sort_by_key(|t| (t.line, t.col));

    let mut data: Vec<SemanticToken> = Vec::with_capacity(toks.len());

    let mut last_line: u32 = 0;
    let mut last_col: u32 = 0;

    for t in toks {
        let delta_line = t.line - last_line;
        let delta_start = if delta_line == 0 {
            t.col - last_col
        } else {
            t.col
        };

        data.push(SemanticToken {
            delta_line,
            delta_start,
            length: t.len,
            token_type: t.ty,
            token_modifiers_bitset: t.mods,
        });

        last_line = t.line;
        last_col = t.col;
    }

    SemanticTokens {
        result_id: None,
        data,
    }
}
