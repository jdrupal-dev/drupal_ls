use lsp_server::{ErrorCode, Request, RequestId, Response, ResponseError};

use super::handlers::completion::handle_text_document_completion;
use super::handlers::code_action::handle_text_document_code_action;
use super::handlers::definition::handle_text_document_definition;
use super::handlers::hover::handle_text_document_hover;

pub fn handle_request(request: Request) -> Response {
    log::trace!("Handling request: {:?}", request);

    let request_id = request.id.clone();
    let response = match request.method.as_str() {
        "textDocument/hover" => handle_text_document_hover(request),
        "textDocument/codeAction" => handle_text_document_code_action(request),
        "textDocument/definition" => handle_text_document_definition(request),
        "textDocument/completion" => handle_text_document_completion(request),
        "shutdown" => None,
        _ => {
            log::warn!("Unhandled request {:?}", request);
            None
        }
    };

    // The LSP spec requires the result to be null for empty responses.
    match response {
        Some(res) => res,
        None => Response {
            id: request_id,
            result: Some(serde_json::Value::Null),
            error: None,
        },
    }
}

pub fn get_response_error(id: RequestId, code: ErrorCode, message: String) -> Response {
    log::error!("Response error: {}", message);

    Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message,
            data: None,
        }),
    }
}
