use tree_sitter::Tree;

pub fn find_label_definition<'a>(
    tree: &'a Tree,
    label_name: &'a str,
    text: &'a str,
) -> Option<tree_sitter::Node<'a>> {
    fn search<'a>(
        node: tree_sitter::Node<'a>,
        label_name: &'a str,
        text: &'a str,
    ) -> Option<tree_sitter::Node<'a>> {

        if node.kind() == "label_definition" {
            // Try ident by field name first
            let id_node = node.child_by_field_name("name")
                .or_else(|| node.child(1));  // Fallback to index 1
            
            if let Some(id_node) = id_node {
                if let Ok(name) = id_node.utf8_text(text.as_bytes()) {
                    if name == label_name {
                        return Some(id_node);
                    }
                }
            }
        }
        
        // Recursively search children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = search(child, label_name, text) {
                return Some(found);
            }
        }
        
        None
    }
    
    search(tree.root_node(), label_name, text)
}

// // Find all references to a label
// pub fn find_label_references<'a>(
//     tree: &'a Tree,
//     label_name: &'a str,
//     text: &'a str,
// ) -> Vec<tree_sitter::Node<'a>> {
//     todo!()
// }