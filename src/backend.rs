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

        let mut docs = self.docs.write().await;
        let doc = match docs.get_mut(&uri) {
            Some(d) => d,
            None => return,
        };

        for change in params.content_changes {
            let mut parser = make_parser();
            doc.update(change.text, version, &mut parser);
        }

        let diags = collect_diagnostics(&doc.tree, doc);

        let log_msg = format!(
            "didChange: {uri} v{version} changes={change_count}, diagnostics={}",
            diags.len()
        );

        self.client.log_message(MessageType::INFO, log_msg).await;
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
                "plop" => {
                    return Ok(Some(make_hover(
                        "PLOP <value>: Push a value onto the stack",
                        r,
                        doc,
                    )));
                }
                "hop" => {
                    return Ok(Some(make_hover("HOP <label>: Conditional jump", r, doc)));
                }
                "leap" => {
                    return Ok(Some(make_hover("LEAP <label>: Unconditional jump", r, doc)));
                }
                "ribbit" => return Ok(Some(make_hover("RIBBIT: Print top of stack", r, doc))),
                "croak" => return Ok(Some(make_hover("CROAK: Read input and push", r, doc))),

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
