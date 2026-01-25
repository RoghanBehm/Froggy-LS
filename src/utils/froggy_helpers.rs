use crate::document::{ByteRange, Index};

pub fn find_label_definition<'a>(index: &'a Index, label_name: &str) -> Option<&'a ByteRange> {
    index.label_defs.get(label_name)
}
