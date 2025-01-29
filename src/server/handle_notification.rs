use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams};
use serde_json::Value;

use crate::document_store::DOCUMENT_STORE;

pub fn handle_notification(notification: Notification) -> () {
    log::trace!("Handling notification: {:?}", notification);

    match notification.method.as_str() {
        "textDocument/didOpen" => handle_text_document_did_open(notification.params),
        "textDocument/didChange" => handle_text_document_did_change(notification.params),
        "textDocument/didClose" => (),
        "textDocument/didSave" => (),
        "exit" => (),
        _ => log::warn!("Unhandled notification {:?}", notification),
    };
}

fn handle_text_document_did_open(params: Value) {
    match serde_json::from_value::<DidOpenTextDocumentParams>(params) {
        Ok(params) => {
            let uri = params.text_document.uri.to_string();
            DOCUMENT_STORE
                .lock()
                .unwrap()
                .add_document(&uri, params.text_document.text);
        }
        Err(err) => log::error!("Could not parse params: {:?}", err),
    }
}

fn handle_text_document_did_change(params: Value) {
    match serde_json::from_value::<DidChangeTextDocumentParams>(params) {
        Ok(params) => {
            let uri = params.text_document.uri.to_string();
            DOCUMENT_STORE
                .lock()
                .unwrap()
                .change_document(&uri, params.content_changes);
        }
        Err(err) => log::error!("Could not parse params: {:?}", err),
    }
}
