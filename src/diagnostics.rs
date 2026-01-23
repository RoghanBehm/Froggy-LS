use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Tree};

fn node_range(node: Node) -> Range {
    let r = node.range();
    Range {
        start: Position {
            line: r.start_point.row as u32,
            character: r.start_point.column as u32,
        },
        end: Position {
            line: r.end_point.row as u32,
            character: r.end_point.column as u32,
        },
    }
}

fn syntax_error_diag(node: Node, text: &str) -> Diagnostic {
    let snippet = node.utf8_text(text.as_bytes()).unwrap_or("__unknown__");
    Diagnostic {
        range: node_range(node),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("froggy".to_string()),
        message: format!("Syntax error near `{snippet}`"),
        ..Default::default()
    }
}

pub fn collect_diagnostics(tree: &Tree, text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];

    while let Some(node) = stack.pop() {
        let is_err = node.is_error() || node.is_missing() || node.kind() == "ERROR";
        if is_err {
            let diag = syntax_error_diag(node, text);

            let r = node.range();
            eprintln!(
                "[DIAG] {}:{}-{}:{} - {}",
                r.start_point.row,
                r.start_point.column,
                r.end_point.row,
                r.end_point.column,
                diag.message
            );

            out.push(diag);
        }

        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }

    out
}
