use line_index::{LineCol, TextSize};
use tower_lsp::lsp_types::{Position, Range};
use tree_sitter::{Node, Tree};

use crate::document::{ByteRange, Doc};

pub fn find_node_at_position<'tree>(
    tree: &'tree Tree,
    doc: &Doc,
    position: Position,
) -> tree_sitter::Node<'tree> {
    let offset = doc.lsp_position_to_offset(position).unwrap_or(0);

    let text_size = TextSize::from(offset as u32);
    let line_col = doc.line_index.line_col(text_size);

    // Byte offset in line
    let line_start_offset = doc
        .line_index
        .offset(LineCol {
            line: line_col.line,
            col: 0,
        })
        .map(|ts| usize::from(ts))
        .unwrap_or(0);
    let column_bytes = offset - line_start_offset;

    let point = tree_sitter::Point {
        row: line_col.line as usize,
        column: column_bytes,
    };

    tree.root_node()
        .descendant_for_point_range(point, point)
        .unwrap_or_else(|| tree.root_node())
}

pub fn labeldef_to_range(def: &ByteRange, doc: &Doc) -> Range {
    let start_offset = def.start;
    let end_offset = def.end;

    Range {
        start: doc.offset_to_lsp_position(start_offset).unwrap_or_default(),
        end: doc.offset_to_lsp_position(end_offset).unwrap_or_default(),
    }
}

pub fn dfs_visit<'tree, F>(tree: &'tree Tree, mut visit: F)
where
    F: FnMut(Node<'tree>),
{
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        visit(node);

        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
}
