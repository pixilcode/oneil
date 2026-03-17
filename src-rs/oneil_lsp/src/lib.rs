// TODO: remove the `allow`s once I have the chance to resolve the issues.
#![allow(clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(dead_code)]

mod diagnostics;
mod doc_store;
mod symbol_lookup;

use std::sync::{Arc, Mutex};

use oneil_runtime::Runtime as OneilRuntime;
use oneil_shared::paths::ModelPath;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, InitializeParams,
    InitializeResult, InitializedParams, MessageType, PositionEncodingKind, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Uri,
};
use tower_lsp_server::{Client, LanguageServer, LspService, Server, UriExt};

use diagnostics::diagnostics_from_runtime_errors;
use doc_store::DocumentStore;

struct Backend {
    client: Client,
    docs: Arc<DocumentStore>,
    // TODO: figure out how to handle async runtime operations better.
    //
    //       Right now, only one thing can use the runtime at a time.
    runtime: Mutex<OneilRuntime>,
}

impl Backend {
    /// Evaluates the model at the given URI and publishes any errors as LSP diagnostics.
    async fn publish_diagnostics_for_model_path(
        &self,
        model_path: &ModelPath,
        version: Option<i32>,
    ) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("publish_diagnostics_for_model_path: {model_path:?}, version: {version:?}"),
            )
            .await;

        let (successful_models, diagnostics) = {
            let mut runtime = self.runtime.lock().expect("runtime mutex poisoned");
            let (result, errors) = runtime.eval_model(model_path);

            let successful_models = result
                .map(|result| result.all_model_paths())
                .unwrap_or_default()
                .into_iter()
                .map(ModelPath::into_path_buf)
                .filter_map(Uri::from_file_path);

            let diagnostics = diagnostics_from_runtime_errors(&errors);

            (successful_models, diagnostics)
        };

        for uri in successful_models {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("clearing diagnostics for successful model: {uri:?}"),
                )
                .await;

            // clear diagnostics for successful models
            self.client
                .publish_diagnostics(uri.clone(), vec![], version)
                .await;
        }

        // publish new diagnostics
        for (uri, diagnostics) in diagnostics {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("publishing diagnostics for {uri:?}: {diagnostics:?}"),
                )
                .await;

            self.client
                .publish_diagnostics(uri, diagnostics, version)
                .await;

            self.client
                .log_message(MessageType::INFO, "diagnostics published".to_string())
                .await;
        }
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "initialize called")
            .await;

        // let params_string = format!("{params:#?}");
        // self.client
        //     .log_message(MessageType::INFO, params_string)
        //     .await;

        let encodings_str = params
            .capabilities
            .general
            .and_then(|general| general.position_encodings)
            .map(|encodings| format!("encodings: {encodings:?}"))
            .unwrap_or_default();
        self.client
            .log_message(MessageType::INFO, encodings_str)
            .await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // VS Code currently expects UTF-16 unless explicitly configured, so advertise UTF-16.
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        open_close: Some(true),
                        ..Default::default()
                    },
                )),
                position_encoding: Some(PositionEncodingKind::UTF16),
                definition_provider: Some(tower_lsp_server::lsp_types::OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "oneil-lsp-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized called")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "shutdown called")
            .await;

        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "did_open called")
            .await;

        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        self.docs.open(params.text_document).await;

        if let Ok(model_path) = ModelPath::try_from(uri.path().as_str()) {
            self.publish_diagnostics_for_model_path(&model_path, Some(version))
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "did_change called")
            .await;

        let result = self
            .docs
            .apply_changes(params.text_document, params.content_changes)
            .await;

        if let Err(error) = result {
            self.client
                .log_message(MessageType::ERROR, format!("did_change error: {error}"))
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "did_close called")
            .await;

        self.docs.close(params.text_document).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "did_save called")
            .await;

        let uri = params.text_document.uri.clone();

        if let Ok(model_path) = ModelPath::try_from(uri.path().as_str()) {
            self.publish_diagnostics_for_model_path(&model_path, None)
                .await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.client
            .log_message(MessageType::INFO, "goto_definition called")
            .await;

        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;

        let Ok(current_model_path) = ModelPath::try_from(uri.path().as_str()) else {
            return Ok(None);
        };

        // Convert LSP position to byte offset
        let Some(offset) = self.docs.position_to_offset(&uri, position).await else {
            self.client
                .log_message(MessageType::WARNING, "Could not convert position to offset")
                .await;

            return Ok(None);
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "goto_definition: offset={}, position={}:{}",
                    offset, position.line, position.character
                ),
            )
            .await;

        // To avoid async problems with holding a mutex guard across an await,
        // we return a tuple of the result and maybe a log message.
        //
        // Each `break 'complete (result, maybe_log_message);` can be thought of as
        // `log(log_message); return result;`
        let (result, maybe_log_message) = 'complete: {
            let mut runtime = self
                .runtime
                .lock()
                .expect("if the runtime has panicked elsewhere, it is not in a useful state");

            let (ir_model, errors) = runtime.load_ir(&current_model_path);

            let Some(ir_model) = ir_model else {
                let errors = errors.to_vec();
                break 'complete (Ok(None), Some(format!("Error loading IR: {errors:?}")));
            };

            // Find the symbol at the cursor position
            let Some(symbol) = symbol_lookup::find_symbol_at_offset(ir_model, offset) else {
                break 'complete (Ok(None), Some("No symbol found at position".to_string()));
            };

            // Resolve the symbol to its definition location
            let location =
                symbol_lookup::resolve_definition(&symbol, &mut runtime, &current_model_path);

            let log_message =
                format!("Found symbol: {symbol:?}, definition location: {location:?}");

            (
                Ok(location.map(GotoDefinitionResponse::Scalar)),
                Some(log_message),
            )
        };

        if let Some(log_message) = maybe_log_message {
            self.client
                .log_message(MessageType::INFO, log_message)
                .await;
        }

        result
    }
}

#[tokio::main]
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let docs = Arc::new(DocumentStore::new());
    let runtime = Mutex::new(OneilRuntime::new());

    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs,
        runtime,
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
