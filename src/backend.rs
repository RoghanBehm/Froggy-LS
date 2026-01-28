use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::diagnostics::collect_diagnostics;
use crate::document::{ByteRange, Doc, make_parser};
use crate::semantic_tokens::{build_semantic_tokens, encode_semantic_tokens, legend};
use crate::utils::froggy_helpers::{
    find_label_definition, find_label_references, leading_word_range, make_hover,
};
use crate::utils::tree_sitter_helpers::{find_node_at_position, labeldef_to_range};

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub docs: Arc<RwLock<HashMap<Url, Doc>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            ..Default::default()
                        },
                    ),
                ),

                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialised!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        let mut parser = make_parser();
        let tree = parser.parse(&text, None).expect("parse() returned None");

        let doc = Doc::new(text, version, tree);

        let diags = collect_diagnostics(&doc.tree, &doc);

        self.client
            .log_message(
                MessageType::INFO,
                format!("didOpen: Found {} diagnostics", diags.len()),
            )
            .await;

        self.client
            .publish_diagnostics(uri.clone(), diags, None)
            .await;

        self.client
            .log_message(
                MessageType::INFO,
                format!("didOpen: {uri} v{version} len={}", doc.text.len()),
            )
            .await;

        self.docs.write().await.insert(uri, doc);
        let _ = self.client.semantic_tokens_refresh().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        let change_count = params.content_changes.len();

        let mut doc = {
            let mut docs = self.docs.write().await;
            match docs.remove(&uri) {
                Some(d) => d,
                None => return,
            }
        };

        for change in params.content_changes {
            let mut parser = make_parser();
            doc.update(change.text, version, &mut parser);
        }

        let diags = collect_diagnostics(&doc.tree, &doc);

        let log_msg = format!(
            "didChange: {uri} v{version} changes={change_count}, diagnostics={}",
            diags.len()
        );

        self.client.log_message(MessageType::INFO, log_msg).await;

        {
            let mut docs = self.docs.write().await;
            docs.insert(uri.clone(), doc);
        }
        self.client.publish_diagnostics(uri, diags, None).await;
        let _ = self.client.semantic_tokens_refresh().await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let node = find_node_at_position(&doc.tree, doc, position);
        let bytes = doc.text.as_bytes();

        let mut cur = node;

        loop {
            let r = leading_word_range(&doc.text, cur);
            match cur.kind() {
                // Stack operations (both the rule names and string literals)
                "PLOP" | "plop" => {
                    return Ok(Some(make_hover(
                        "PLOP <value>: Push a value onto the stack",
                        r,
                        doc,
                    )));
                }
                "SPLASH" | "splash" => {
                    return Ok(Some(make_hover(
                        "SPLASH: Pop a value off the stack",
                        r,
                        doc,
                    )));
                }
                "GULP" | "gulp" => {
                    return Ok(Some(make_hover("GULP: Increment top of stack", r, doc)));
                }
                "BURP" | "burp" => {
                    return Ok(Some(make_hover("BURP: Decrement top of stack", r, doc)));
                }
                "DUP" | "dup" => {
                    return Ok(Some(make_hover("DUP: Duplicate top of stack", r, doc)));
                }
                "SWAP" | "swap" => {
                    return Ok(Some(make_hover("SWAP: Swap top two stack values", r, doc)));
                }
                "OVER" | "over" => {
                    return Ok(Some(make_hover(
                        "OVER: Duplicate second from top of stack",
                        r,
                        doc,
                    )));
                }

                // Control flow (both rule names and string literals)
                "LILY" | "lily" => {
                    return Ok(Some(make_hover(
                        "LILY <label>: Define a lilypad label",
                        r,
                        doc,
                    )));
                }
                "HOP" | "hop" => {
                    return Ok(Some(make_hover(
                        "HOP <Lilypad>: Unconditional jump to a lilypad",
                        r,
                        doc,
                    )));
                }
                "LEAP" | "leap" => {
                    return Ok(Some(make_hover(
                        "LEAP <Lilypad>: Pop a, if (a == 0) then jump to lilypad",
                        r,
                        doc,
                    )));
                }

                // Label definition
                "label_definition" => {
                    if let Some(label_node) = cur.child_by_field_name("name") {
                        let label_text = label_node.utf8_text(bytes).unwrap_or("");
                        let label_range = ByteRange {
                            start: label_node.start_byte(),
                            end: label_node.end_byte(),
                        };
                        return Ok(Some(make_hover(
                            &format!("Label definition: {}", label_text),
                            label_range,
                            doc,
                        )));
                    }
                }

                // IO (both rule names and string literals)
                "RIBBIT" | "ribbit" => {
                    return Ok(Some(make_hover("RIBBIT: Print top of stack", r, doc)));
                }
                "CROAK" | "croak" => return Ok(Some(make_hover("CROAK: Read input", r, doc))),

                // Arithmetic (both rule names and string literals)
                "ADD" | "add" => return Ok(Some(make_hover("ADD: Pop a b, push (b + a)", r, doc))),
                "SUB" | "sub" => return Ok(Some(make_hover("SUB: Pop a b, push (b - a)", r, doc))),
                "MUL" | "mul" => return Ok(Some(make_hover("MUL: Pop a b, push (b * a)", r, doc))),
                "DIV" | "div" => return Ok(Some(make_hover("DIV: Pop a b, push (b / a)", r, doc))),

                // Comparison (both rule names and string literals)
                "EQUALS" | "equals" => {
                    return Ok(Some(make_hover("EQUALS: Pop a b, push (b == a)", r, doc)));
                }
                "NOT_EQUAL" | "not_equal" => {
                    return Ok(Some(make_hover(
                        "NOT_EQUAL: Pop a b, push (b != a)",
                        r,
                        doc,
                    )));
                }
                "LESS_THAN" | "less_than" => {
                    return Ok(Some(make_hover("LESS_THAN: Pop a b, push (b < a)", r, doc)));
                }
                "GREATER_THAN" | "greater_than" => {
                    return Ok(Some(make_hover(
                        "GREATER_THAN: Pop a b, push (b > a)",
                        r,
                        doc,
                    )));
                }
                "LESS_EQ" | "less_eq" => {
                    return Ok(Some(make_hover("LESS_EQ: Pop a b, push (b <= a)", r, doc)));
                }
                "GREATER_EQ" | "greater_eq" => {
                    return Ok(Some(make_hover(
                        "GREATER_EQ: Pop a b, push (b >= a)",
                        r,
                        doc,
                    )));
                }
                "identifier" => {
                    if let Some(parent) = cur.parent() {
                        if matches!(parent.kind(), "label_definition" | "hop" | "leap") {
                            // Continue to parent instead of returning
                        } else {
                            let text = cur.utf8_text(bytes).unwrap_or("");
                            if let Some(_def) = find_label_definition(&doc.index, text) {
                                return Ok(Some(make_hover(&format!("Label: {}", text), r, doc)));
                            }
                        }
                    }
                }

                _ => {}
            }
            // Break out of loop if node is parentless
            match cur.parent() {
                Some(p) => cur = p,
                None => break,
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let node = find_node_at_position(&doc.tree, doc, position);

        if node.kind() == "identifier" {
            if let Some(parent) = node.parent() {
                if parent.kind() == "hop" || parent.kind() == "leap" {
                    let label_name = node.utf8_text(doc.text.as_bytes()).unwrap_or("__unknown__");

                    if let Some(def_node) = find_label_definition(&doc.index, label_name) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri: uri.clone(),
                            range: labeldef_to_range(def_node, doc),
                        })));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        Ok(Some(SemanticTokensResult::Tokens(encode_semantic_tokens(
            build_semantic_tokens(doc),
        ))))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.docs.read().await;
        let doc = match docs.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let node = find_node_at_position(&doc.tree, doc, position);

        if node.kind() == "identifier" {
            let label_name = node.utf8_text(doc.text.as_bytes()).unwrap_or("__unknown__");
            let mut locations: Vec<Location> = Vec::new();

            // Add def if exists
            if params.context.include_declaration {
                if let Some(def) = find_label_definition(&doc.index, label_name) {
                    locations.push(Location::new(uri.clone(), labeldef_to_range(def, doc)));
                }
            }

            // Add refs
            if let Some(refs) = find_label_references(&doc.index, label_name) {
                for r in refs {
                    let ltr = labeldef_to_range(r, doc);
                    locations.push(Location::new(uri.clone(), ltr));
                }
            }
            if !locations.is_empty() {
                return Ok(Some(locations));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
    &self,
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let uri = &params.text_document.uri;
    let docs = self.docs.read().await;
    let doc = match docs.get(uri) {
        Some(d) => d,
        None => return Ok(None),
    };

#[allow(deprecated)]
let symbols: Vec<DocumentSymbol> = doc.index.label_defs
    .iter()
    .map(|(name, range)| DocumentSymbol {
        name: name.clone(),
        detail: Some("Label".to_string()),
        kind: SymbolKind::FUNCTION,
        range: labeldef_to_range(range, doc),
        selection_range: labeldef_to_range(range, doc),
        children: None,
        tags: None,
        deprecated: None,
    })
    .collect();

    Ok(Some(DocumentSymbolResponse::Nested(symbols)))
}
}
