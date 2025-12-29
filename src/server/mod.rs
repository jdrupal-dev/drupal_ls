mod handle_notification;
mod handle_request;
mod handlers;

use std::net::{Ipv4Addr, SocketAddrV4};
use std::vec;

use anyhow::Result;
use lsp_server::{Connection, Message};
use lsp_types::{
    CompletionOptions, HoverProviderCapability, InitializeParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};

use crate::document_store::initialize_document_store;
use crate::opts::DrupalLspConfig;
use crate::utils::uri_to_url;

use self::handle_notification::handle_notification;
use self::handle_request::handle_request;

async fn main_loop(connection: Connection) {
    for msg in &connection.receiver {
        match msg {
            Message::Notification(notification) => handle_notification(notification),
            Message::Request(request) => {
                let response = handle_request(request);
                if let Err(e) = connection.sender.send(Message::Response(response)) {
                    log::error!("Failed to send response: {:?}", e);
                }
            }
            _ => log::error!("Unable to process message: {:?}", msg),
        };
    }
}

pub async fn start_lsp(config: DrupalLspConfig) -> Result<()> {
    // Note that we must have our logging only write out to stderr.
    log::info!("Starting Drupal Language server");

    let (connection, io_threads);
    if let Some(socket_port) = config.socket.or(config.port) {
        (connection, io_threads) =
            Connection::connect(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), socket_port))?;
    } else if config.pipe.is_some() {
        panic!("Unsupported transport type 'pipe'.");
    } else {
        (connection, io_threads) = Connection::stdio();
    }

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["@".to_string(), " ".to_string()]),
            ..CompletionOptions::default()
        }),
        ..Default::default()
    })
    .unwrap();

    let initialize_params = connection.initialize(server_capabilities)?;
    let initialize_params: InitializeParams = serde_json::from_value(initialize_params).unwrap();

    if let Some(folders) = initialize_params.workspace_folders {
        let uri = folders
            .first()
            .expect("Unable to initialize without a workspace folder")
            .uri
            .clone();
        if let Some(url) = uri_to_url(uri) {
            // Start non-blocking document store initialization.
            tokio::spawn(async move {
                initialize_document_store(url);
            });
        }
    }

    main_loop(connection).await;

    io_threads.join()?;

    // Shut down gracefully.
    log::info!("Shutting down Drupal LSP server");
    Ok(())
}
