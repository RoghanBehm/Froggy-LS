use tree_sitter::{Parser, Tree};

#[derive(Debug)]
pub struct Doc {
    pub text: String,
    pub version: i32,
    pub tree: Tree,
}

impl Doc {
    pub fn new(text: String, version: i32, tree: Tree) -> Self {
        Self {
            text,
            version,
            tree,
        }
    }

    pub fn update(&mut self, text: String, version: i32, parser: &mut Parser) {
        self.text = text;
        self.version = version;
        self.tree = parser
            .parse(&self.text, Some(&self.tree))
            .expect("parse() returned None");
    }
}

pub fn make_parser() -> Parser {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_froggy::LANGUAGE.into())
        .expect("load language");
    p
}
