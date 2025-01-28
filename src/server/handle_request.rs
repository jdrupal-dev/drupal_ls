use lsp_server::{ErrorCode, Request, RequestId, Response, ResponseError};

use super::handlers::completion::handle_text_document_completion;
use super::handlers::definition::handle_text_document_definition;
use super::handlers::hover::handle_text_document_hover;

pub fn handle_request(request: Request) -> Option<Response> {
    log::trace!("Handling request: {:?}", request);

    match request.method.as_str() {
        "textDocument/hover" => handle_text_document_hover(request),
        "textDocument/definition" => handle_text_document_definition(request),
        "textDocument/completion" => handle_text_document_completion(request),
        _ => {
            log::warn!("Unhandled request {:?}", request);
            None
        }
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
