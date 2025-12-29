use std::collections::HashMap;

use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, Position, Range, TextEdit, Uri, WorkspaceEdit,
};
use regex::Regex;

use crate::{
    document_store::DOCUMENT_STORE,
    parser::tokens::{Token, TokenData},
    server::handle_request::get_response_error,
};

pub fn handle_text_document_code_action(request: Request) -> Option<Response> {
    let params = match serde_json::from_value::<CodeActionParams>(request.params) {
        Err(err) => {
            return Some(get_response_error(
                request.id,
                ErrorCode::InvalidParams,
                format!("Could not parse code action params: {:?}", err),
            ));
        }
        Ok(value) => value,
    };

    let mut token: Option<Token> = None;
    if let Some(document) = DOCUMENT_STORE
        .lock()
        .unwrap()
        .get_document(&params.text_document.uri.to_string())
    {
        token = document.get_token_under_cursor(params.range.start);
    }

    let mut code_actions_result: Vec<CodeAction> = vec![];
    if let Some(token) = token {
        if let TokenData::DrupalTranslationString(token_data) = &token.data {
            let re = Regex::new(r#"(?<placeholder>[@%:]\w*)"#).unwrap();
            let arguments_string: String = format!(
                ", [{}]",
                re.captures_iter(&token_data.string)
                    .map(|capture| capture.name("placeholder"))
                    .filter_map(|str| Some(format!("'{}' => ''", str?.as_str())))
                    .collect::<Vec<String>>()
                    .join(", ")
            );

            let mut text_edits: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
            text_edits.insert(
                params.text_document.uri,
                vec![TextEdit {
                    range: Range {
                        start: Position {
                            line: token.range.end_point.row as u32,
                            character: token.range.end_point.column as u32 - 1,
                        },
                        end: Position {
                            line: token.range.end_point.row as u32,
                            character: token.range.end_point.column as u32 - 1,
                        },
                    },
                    new_text: arguments_string,
                }],
            );

            code_actions_result.push(CodeAction {
                title: String::from("Add translations placeholders"),
                kind: Some(CodeActionKind::REFACTOR_INLINE),
                diagnostics: None,
                edit: Some(WorkspaceEdit {
                    changes: Some(text_edits),
                    document_changes: None,
                    change_annotations: None,
                }),
                command: None,
                is_preferred: Some(true),
                disabled: None,
                data: None,
            });
        }
    }

    match serde_json::to_value(code_actions_result) {
        Ok(result) => Some(Response {
            id: request.id,
            result: Some(result),
            error: None,
        }),
        Err(error) => Some(get_response_error(
            request.id,
            ErrorCode::InternalError,
            format!("No code actions found: {:?}", error),
        )),
    }
}
