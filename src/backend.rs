use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::diagnostics::collect_diagnostics;
use crate::document::{Doc, make_parser};
use crate::semantic_tokens::{build_semantic_tokens, encode_semantic_tokens, legend};
use crate::utils::froggy_helpers::{find_label_definition, leading_word_range, make_hover};
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

                // Stack
                "plop" => return Ok(Some(make_hover("PLOP <value>: Push a value onto the stack", r, doc,))),
                "splash" => return Ok(Some(make_hover("SPLASH <Lilypad>: Pop a value off the stack", r, doc))),
                "gulp" => return Ok(Some(make_hover("GULP: Increment top of stack", r, doc))),
                "burp" => return Ok(Some(make_hover("BURP: Decrement top of stack", r, doc))),
                "dup" => return Ok(Some(make_hover("DUP: Duplicate top of stack", r, doc))),
                "swap" => return Ok(Some(make_hover("SWAP: Swap the top of the stack with its predecessor", r, doc))),
                "over" => return Ok(Some(make_hover("OVER: Duplicate second from top of stack", r, doc))),

                 // Control flow   
                "lily" => return Ok(Some(make_hover("LILY <label>: lilypad", r, doc))),
                "hop" => return Ok(Some(make_hover("HOP <Lilypad>: Unconditional jump to a lilypad", r, doc))),
                "leap" => return Ok(Some(make_hover("LEAP <Lilypad>: Pop a, if (a == 0) then jump to lilypad", r, doc))),

                // IO
                "ribbit" => return Ok(Some(make_hover("RIBBIT: Print top of stack", r, doc))),
                "croak" => return Ok(Some(make_hover("CROAK: Not implemented", r, doc))),


                // Arithmetic
                "add" => return Ok(Some(make_hover("ADD: Pop a b, push (b + a)", r, doc))),
                "sub" => return Ok(Some(make_hover("SUB: Pop a b, push (b - a)", r, doc))),
                "mul" => return Ok(Some(make_hover("MUL: Pop a b, push (b * a)", r, doc))),
                "div" => return Ok(Some(make_hover("DIV: Pop a b, push (b / a)", r, doc))),

                // Comparison
                "equals" => return Ok(Some(make_hover("EQUALS: Pop a b, push (b == a)", r, doc))),
                "not_equal" => return Ok(Some(make_hover("NOT_EQUAL: Pop a b, push (b != a)", r, doc))),
                "less_than" => return Ok(Some(make_hover("LESS_THAN: Pop a b, push (b < a)", r, doc))),
                "greater_than" => return Ok(Some(make_hover("GREATER_THAN: Pop a b, push (b > a)", r, doc))),
                "less_eq" => return Ok(Some(make_hover("LESS_EQ: Pop a b, push (b <= a)", r, doc))),
                "greater_eq" => return Ok(Some(make_hover("GREATER_EQ: Pop a b, push (b >= a)", r, doc))),
                "identifier" => {
                    let text = cur.utf8_text(bytes).unwrap_or("");
                    if let Some(_def) = find_label_definition(&doc.index, text) {
                        return Ok(Some(make_hover(&format!("Label: {}", text), r, doc)));
                    } else {
                        return Ok(None);
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
}
