use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::diagnostics::collect_diagnostics;
use crate::document::{make_parser, Doc};

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
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        
        let mut parser = make_parser();
        let tree = parser.parse(&text, None).expect("parse() returned None");
        let diags = collect_diagnostics(&tree, &text);
        
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
                format!("didOpen: {uri} v{version} len={}", text.len()),
            )
            .await;

        self.docs
            .write()
            .await
            .insert(uri, Doc::new(text, version, tree));
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        let change_count = params.content_changes.len();

        let (diags, log_msg) = {
            let mut docs = self.docs.write().await;
            let doc = match docs.get_mut(&uri) {
                Some(d) => d,
                None => return,
            };

            for change in params.content_changes {
                let mut parser = make_parser();
                doc.update(change.text, version, &mut parser);
            }

            let diags = collect_diagnostics(&doc.tree, &doc.text);
            let log_msg = format!("didChange: {uri} v{version} changes={change_count}, diagnostics={}", diags.len());

            (diags, log_msg)
        };

        self.client.log_message(MessageType::INFO, log_msg).await;
        self.client.publish_diagnostics(uri, diags, None).await;
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

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "You're hovering!".to_string(),
            )),
            range: None,
        }))
    }
}