use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString};
use crate::utils::tree_sitter_helpers::labeldef_to_range;
use crate::document::{ByteRange, Doc, Index};

pub fn find_label_definition<'a>(index: &'a Index, label_name: &str) -> Option<&'a ByteRange> {
    index.label_defs.get(label_name)
}

pub fn make_hover(msg: &str, node: tree_sitter::Node, doc: &Doc) -> Hover {
    Hover {
        contents: HoverContents::Scalar(MarkedString::String(msg.to_string())),
        range: Some(labeldef_to_range(
            &ByteRange { start: node.start_byte(), end: node.end_byte() },
            doc,
        )),
    }
}
