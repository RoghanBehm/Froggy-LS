use tower_lsp::lsp_types::{Position, Range};
use tree_sitter::Tree;

pub fn find_node_at_position(tree: &Tree, position: Position) -> tree_sitter::Node<'_> {
    let point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };
    
    tree.root_node()
        .descendant_for_point_range(point, point)
        .unwrap_or_else(|| tree.root_node())
}

// Convert a tree-sitter node to an LSP Range
pub fn node_to_range(node: tree_sitter::Node) -> Range {
    Range {
        start: Position {
            line: node.start_position().row as u32,
            character: node.start_position().column as u32,
        },
        end: Position {
            line: node.end_position().row as u32,
            character: node.end_position().column as u32,
        },
    }
}