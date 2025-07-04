use std::collections::HashMap;
use std::sync::Arc;

use braise_core::{ast::*, error::BraiseError};
use lexer::{SpannedToken, tokenize};
use miette::Result;
use parser::Parser;
use ropey::Rope;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url::Url;

mod diagnostics;
mod symbols;
mod text_document;

use diagnostics::DiagnosticsProvider;
use symbols::SymbolProvider;
use text_document::TextDocumentProvider;

#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub content: Rope,
    pub version: i32,
    pub ast: Option<Config>,
    pub tokens: Option<Vec<SpannedToken>>,
}

fn snake_case_to_camel_case(name: &str) -> String {
    name.split('_')
        .map(|s| s.chars().next().unwrap().to_uppercase().to_string() + &s[1..])
        .collect::<Vec<_>>()
        .join("")
}

impl Document {
    pub fn new(uri: Url, content: String, version: i32) -> Self {
        Self {
            uri,
            content: Rope::from_str(&content),
            version,
            ast: None,
            tokens: None,
        }
    }

    pub fn update_content(&mut self, content: String, version: i32) {
        self.content = Rope::from_str(&content);
        self.version = version;
        self.ast = None;
        self.tokens = None;
    }

    pub fn get_text(&self) -> String {
        self.content.to_string()
    }

    pub fn parse(&mut self) -> Result<(), BraiseError> {
        let text = self.get_text();

        let tokens = tokenize(&text)?;

        let mut parser = Parser::new(tokens.clone(), text, self.uri.to_string());
        let ast = parser.parse()?;

        self.tokens = Some(tokens);
        self.ast = Some(ast);

        Ok(())
    }
}

pub struct BraiseLspServer {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, Document>>>,

    diagnostics_provider: DiagnosticsProvider,
    symbol_provider: SymbolProvider,
    text_document_provider: TextDocumentProvider,
}

impl BraiseLspServer {
    pub fn new(client: Client) -> Self {
        Self {
            client: client.clone(),
            documents: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_provider: DiagnosticsProvider::new(client.clone()),
            symbol_provider: SymbolProvider::new(),
            text_document_provider: TextDocumentProvider::new(),
        }
    }

    async fn get_document(&self, uri: &Url) -> Option<Document> {
        self.documents.read().await.get(uri).cloned()
    }

    async fn parse_and_publish_diagnostics(&self, uri: &Url) {
        if let Some(mut doc) = self.get_document(uri).await {
            if let Err(error) = doc.parse() {
                self.diagnostics_provider
                    .publish_parse_error(uri, &error)
                    .await;
            } else if let Some(ref ast) = doc.ast {
                let diagnostics = self.diagnostics_provider.validate_ast(ast, &doc);
                self.diagnostics_provider
                    .publish_diagnostics(uri, diagnostics)
                    .await;
            } else {
                self.diagnostics_provider.clear_diagnostics(uri).await;
            }

            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), doc);
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BraiseLspServer {
    async fn initialize(&self, _params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        tracing::info!("Braise LSP Server initializing...");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: None,
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::COMMENT,
                                ],
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: format!(
                    "{} LSP Server",
                    snake_case_to_camel_case(env!("CARGO_PKG_NAME"))
                ),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Braise LSP Server initialized!");

        self.client
            .log_message(MessageType::INFO, "Braise LSP Server initialized")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        tracing::info!("Braise LSP Server shutting down...");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        let document = Document::new(uri.clone(), content, version);

        {
            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), document);
        }

        self.parse_and_publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().next()
            && let Some(mut doc) = self.get_document(&uri).await
        {
            doc.update_content(change.text, version);

            {
                let mut documents = self.documents.write().await;
                documents.insert(uri.clone(), doc);
            }

            self.parse_and_publish_diagnostics(&uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut documents = self.documents.write().await;
        documents.remove(&params.text_document.uri);

        // Clear diagnostics
        self.diagnostics_provider
            .clear_diagnostics(&params.text_document.uri)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        if let Some(doc) = self.get_document(&uri).await {
            let hover = self
                .text_document_provider
                .provide_hover(&doc, position)
                .await;
            Ok(hover)
        } else {
            Ok(None)
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        if let Some(doc) = self.get_document(&uri).await {
            let definition = self
                .text_document_provider
                .provide_definition(&doc, position)
                .await;
            Ok(definition.map(GotoDefinitionResponse::Scalar))
        } else {
            Ok(None)
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        if let Some(doc) = self.get_document(&uri).await {
            let symbols = self.symbol_provider.provide_document_symbols(&doc).await;
            Ok(Some(DocumentSymbolResponse::Nested(symbols)))
        } else {
            Ok(None)
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        let query = params.query;
        let documents = self.documents.read().await;

        let symbols = self
            .symbol_provider
            .provide_workspace_symbols(&documents, &query)
            .await;
        Ok(Some(symbols))
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;

        if let Some(doc) = self.get_document(&uri).await {
            let actions = self
                .text_document_provider
                .provide_code_actions(&doc, range)
                .await;
            Ok(Some(actions))
        } else {
            Ok(None)
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        if let Some(doc) = self.get_document(&uri).await {
            let edits = self.text_document_provider.provide_formatting(&doc).await;
            Ok(Some(edits))
        } else {
            Ok(None)
        }
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;

        if let Some(doc) = self.get_document(&uri).await {
            let ranges = self
                .text_document_provider
                .provide_folding_ranges(&doc)
                .await;
            Ok(Some(ranges))
        } else {
            Ok(None)
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        if let Some(doc) = self.get_document(&uri).await {
            let tokens = self
                .text_document_provider
                .provide_semantic_tokens(&doc)
                .await;
            Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: tokens.map_or_else(Vec::new, |e| e.data),
            })))
        } else {
            Ok(None)
        }
    }
}
