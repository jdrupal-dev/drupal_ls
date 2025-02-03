use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Position, Range};

use crate::document_store::DOCUMENT_STORE;
use crate::parser::tokens::{Token, TokenData};
use crate::server::handle_request::get_response_error;

pub fn handle_text_document_definition(request: Request) -> Option<Response> {
    let params = match serde_json::from_value::<GotoDefinitionParams>(request.params) {
        Err(err) => {
            return Some(get_response_error(
                request.id,
                ErrorCode::InvalidParams,
                format!("Could not parse definition params: {:?}", err),
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

    let Some(token) = token else {
        return Some(Response {
            id: request.id,
            result: Some(serde_json::Value::Null),
            error: None,
        });
    };

    let Some(definition_result) = provide_definition_for_token(&token) else {
        return None;
    };

    return match serde_json::to_value(definition_result) {
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
    };
}

fn provide_definition_for_token(token: &Token) -> Option<GotoDefinitionResponse> {
    let store = DOCUMENT_STORE.lock().unwrap();

    let (source_document, token) = match &token.data {
        TokenData::PhpClassReference(class) => store.get_class_definition(class),
        TokenData::PhpMethodReference(method) => store.get_method_definition(method),
        TokenData::DrupalServiceReference(service_name) => {
            store.get_service_definition(service_name)
        }
        TokenData::DrupalRouteReference(route_name) => store.get_route_definition(route_name),
        TokenData::DrupalHookReference(hook_name) => store.get_hook_definition(hook_name),
        TokenData::DrupalPermissionReference(permission_name) => store.get_permission_definition(permission_name),
        _ => None,
    }?;

    return Some(GotoDefinitionResponse::Scalar(lsp_types::Location {
        uri: source_document.get_uri()?,
        range: Range {
            start: Position {
                line: token.range.start_point.row as u32,
                character: token.range.start_point.column as u32,
            },
            end: Position {
                line: token.range.end_point.row as u32,
                character: token.range.end_point.column as u32,
            },
        },
    }));
}
