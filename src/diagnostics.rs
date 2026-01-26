use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Tree};

use crate::document::Doc;

fn node_range(node: Node, doc: &Doc) -> Range {
    Range {
        start: doc
            .offset_to_lsp_position(node.start_byte())
            .unwrap_or_default(),
        end: doc
            .offset_to_lsp_position(node.end_byte())
            .unwrap_or_default(),
    }
}

fn syntax_error_diag(node: Node, doc: &Doc) -> Diagnostic {
    let snippet = node.utf8_text(doc.text.as_bytes()).unwrap_or("__unknown__");
    Diagnostic {
        range: node_range(node, doc),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("froggy".to_string()),
        message: format!("Syntax error near `{snippet}`"),
        ..Default::default()
    }
}

pub fn collect_diagnostics(tree: &Tree, doc: &Doc) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];

    eprintln!("=== DEBUG: Starting diagnostic collection ===");
    eprintln!("Tree root kind: {}", tree.root_node().kind());
    eprintln!("Tree has_error: {}", tree.root_node().has_error());

    while let Some(node) = stack.pop() {
        let is_err = node.is_error() || node.is_missing() || node.kind() == "ERROR";
        if is_err {
            eprintln!(">>> FOUND ERROR NODE: {}", node.kind());
            out.push(syntax_error_diag(node, doc));
        }

        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }

    eprintln!("=== Total diagnostics found: {} ===", out.len());
    out
}
