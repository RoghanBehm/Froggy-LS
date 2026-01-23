use tower_lsp::lsp_types::*;
use tree_sitter::Tree;

pub fn collect_diagnostics(tree: &Tree, text: &str) -> Vec<Diagnostic> {
    let root = tree.root_node();
    let mut out = Vec::new();

    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.is_error() || n.is_missing() || n.kind() == "ERROR" {
            let r = n.range();
            let diagnostic = Diagnostic {
                range: Range {
                    start: Position {
                        line: r.start_point.row as u32,
                        character: r.start_point.column as u32,
                    },
                    end: Position {
                        line: r.end_point.row as u32,
                        character: r.end_point.column as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("froggy".to_string()),
                message: format!(
                    "Syntax error near `{}`",
                    n.utf8_text(text.as_bytes()).unwrap_or("")
                ),
                ..Default::default()
            };
            eprintln!(
                "[DIAG] {}:{}-{}:{} - {}",
                r.start_point.row,
                r.start_point.column,
                r.end_point.row,
                r.end_point.column,
                diagnostic.message
            );
            out.push(diagnostic);
        }

        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }

    out
}