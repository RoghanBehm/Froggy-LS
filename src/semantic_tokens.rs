use tower_lsp::lsp_types::SemanticTokens;
use tower_lsp::lsp_types::*;

use crate::document::{ByteRange, Doc};
use crate::utils::froggy_helpers::leading_word_range;
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
            SemanticTokenType::KEYWORD,   // 0 - control flow
            SemanticTokenType::NUMBER,    // 1
            SemanticTokenType::COMMENT,   // 2
            SemanticTokenType::STRING,    // 3
            SemanticTokenType::VARIABLE,  // 4 - labels
            SemanticTokenType::FUNCTION,  // 5 - stack operations
            SemanticTokenType::OPERATOR,  // 6 - arithmetic/comparison
            SemanticTokenType::PARAMETER, // 7 - I/O operations
        ],
        token_modifiers: vec![SemanticTokenModifier::DEFINITION],
    }
}

// token type indices
pub mod token_types {
    pub const KEYWORD: u32 = 0; // control flow
    pub const NUMBER: u32 = 1;
    pub const COMMENT: u32 = 2;
    pub const STRING: u32 = 3;
    pub const VARIABLE: u32 = 4; // labels
    pub const FUNCTION: u32 = 5; // stack ops
    pub const OPERATOR: u32 = 6; // arithmetic/comparison
    pub const PARAMETER: u32 = 7; // I/O ops
}

// modifier bitset flags
pub mod token_modifiers {
    pub const DEFINITION: u32 = 1 << 0; // index 1
}

// Create ByteRange from a node
fn node_range(node: tree_sitter::Node) -> ByteRange {
    ByteRange {
        start: node.start_byte(),
        end: node.end_byte(),
    }
}

// Add token for node's leading word
fn add_token(toks: &mut Vec<Tok>, doc: &Doc, node: tree_sitter::Node, ty: u32, mods: u32) {
    let r = leading_word_range(&doc.text, node);
    if let Some(t) = tok_from_range(doc, &r, ty, mods) {
        toks.push(t);
    }
}

// Add token for specific byte range
fn add_token_range(toks: &mut Vec<Tok>, doc: &Doc, range: ByteRange, ty: u32, mods: u32) {
    if let Some(t) = tok_from_range(doc, &range, ty, mods) {
        toks.push(t);
    }
}

pub fn build_semantic_tokens(doc: &Doc) -> Vec<Tok> {
    let mut toks = Vec::new();

    dfs_visit(&doc.tree, |node| {
        let kind = node.kind();

        // Control flow keywords
        if matches!(kind, "lily" | "hop" | "leap") {
            add_token(&mut toks, doc, node, token_types::KEYWORD, 0);

            return;
        }

        // Stack manipulation operations
        if matches!(
            kind,
            "plop" | "splash" | "gulp" | "burp" | "dup" | "swap" | "over"
        ) {
            add_token(&mut toks, doc, node, token_types::FUNCTION, 0);
        }

        // Arithmetic and comparison operators
        if matches!(
            kind,
            "add"
                | "sub"
                | "mul"
                | "div"
                | "less_than"
                | "greater_than"
                | "equals"
                | "not_equal"
                | "less_eq"
                | "greater_eq"
        ) {
            add_token(&mut toks, doc, node, token_types::OPERATOR, 0);
        }

        // I/O operations
        if matches!(kind, "ribbit" | "croak") {
            add_token(&mut toks, doc, node, token_types::PARAMETER, 0);
            return;
        }

        match kind {
            "label_definition" => {
                // LILY keyword
                if let Some(keyword_node) = node.child(0) {
                    add_token_range(
                        &mut toks,
                        doc,
                        node_range(keyword_node),
                        token_types::KEYWORD,
                        0,
                    );
                }

                // Label name
                if let Some(name_node) = node.child_by_field_name("name") {
                    add_token_range(
                        &mut toks,
                        doc,
                        node_range(name_node),
                        token_types::VARIABLE,
                        token_modifiers::DEFINITION,
                    );
                }
            }

            "hop" | "leap" => {
                // Label identifier
                if let Some(target_node) = node.child_by_field_name("target") {
                    add_token_range(
                        &mut toks,
                        doc,
                        node_range(target_node),
                        token_types::VARIABLE,
                        0,
                    );
                }
            }

            "identifier" => {
                // Skip identifiers hop/leap/label_definition idents
                if let Some(parent) = node.parent() {
                    if matches!(parent.kind(), "label_definition" | "hop" | "leap") {
                        return;
                    }
                }
                add_token_range(&mut toks, doc, node_range(node), token_types::VARIABLE, 0);
            }

            "number" | "integer" | "float" => {
                add_token_range(&mut toks, doc, node_range(node), token_types::NUMBER, 0);
            }

            "string" | "string_literal" => {
                add_token_range(&mut toks, doc, node_range(node), token_types::STRING, 0);
            }

            "comment" | "line_comment" | "block_comment" => {
                add_token_range(&mut toks, doc, node_range(node), token_types::COMMENT, 0);
            }
            _ => {}
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
