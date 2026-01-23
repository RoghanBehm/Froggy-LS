use std::{env, fs, process};
use tree_sitter::Parser;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --bin parse -- <file.frog>");
        process::exit(2);
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", path, e);
        process::exit(2);
    });

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_froggy::LANGUAGE.into())
        .expect("Error loading Froggy parser");

    let tree = parser
        .parse(&source, None)
        .expect("tree-sitter returned None");
    let root = tree.root_node();

    println!("has_error: {}", root.has_error());
    println!("{}", root.to_sexp());
}
