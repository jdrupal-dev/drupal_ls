use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{Hover, HoverContents, HoverParams};

use crate::document_store::DOCUMENT_STORE;
use crate::documentation::get_documentation_for_token;
use crate::parser::tokens::Token;
use crate::server::handle_request::get_response_error;

pub fn handle_text_document_hover(request: Request) -> Option<Response> {
    let params = match serde_json::from_value::<HoverParams>(request.params) {
        Err(err) => {
            return Some(get_response_error(
                request.id,
                ErrorCode::InvalidParams,
                format!("Could not parse hover params: {:?}", err),
            ));
        }
        Ok(value) => value,
    };

    let mut token: Option<Token> = None;
    if let Some(document) = DOCUMENT_STORE.lock().unwrap().get_document(
        &params
            .text_document_position_params
            .text_document
            .uri
            .to_string(),
    ) {
        token = document.get_token_under_cursor(params.text_document_position_params.position);
    }

    let hover_result = Hover {
        contents: HoverContents::Scalar(lsp_types::MarkedString::String(
            get_documentation_for_token(&token?)?,
        )),
        range: None,
    };

    match serde_json::to_value(hover_result) {
        Ok(result) => Some(Response {
            id: request.id,
            result: Some(result),
            error: None,
        }),
        Err(error) => Some(get_response_error(
            request.id,
            ErrorCode::InternalError,
            format!("No hover found: {:?}", error),
        )),
    }
}
