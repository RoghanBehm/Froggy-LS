use crate::document::{ByteRange, Doc, Index};
use crate::utils::tree_sitter_helpers::labeldef_to_range;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString};
use tree_sitter::Node;

pub fn find_label_definition<'a>(index: &'a Index, label_name: &str) -> Option<&'a ByteRange> {
    index.label_defs.get(label_name)
}

pub fn find_label_references<'a>(index: &'a Index, label_name: &str) -> Option<&'a Vec<ByteRange>> {
    index.label_refs.get(label_name)
}

pub fn make_hover(msg: &str, range: ByteRange, doc: &Doc) -> Hover {
    Hover {
        contents: HoverContents::Scalar(MarkedString::String(msg.to_string())),
        range: Some(labeldef_to_range(&range, doc)),
    }
}

pub fn leading_word_range(doc_text: &str, node: Node) -> ByteRange {
    let bytes = doc_text.as_bytes();
    let start = node.start_byte();
    let mut end = start;

    while end < bytes.len() {
        match bytes[end] {
            b' ' | b'\t' | b'\r' | b'\n' => break,
            _ => end += 1,
        }
    }

    ByteRange { start, end }
}
