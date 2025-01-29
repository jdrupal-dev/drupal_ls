use std::collections::HashMap;

use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionList, CompletionParams, Documentation, InsertTextFormat
};
use regex::Regex;

use crate::document_store::DOCUMENT_STORE;
use crate::documentation::get_documentation_for_token;
use crate::parser::tokens::{Token, TokenData};
use crate::server::handle_request::get_response_error;

pub fn handle_text_document_completion(request: Request) -> Option<Response> {
    let params = match serde_json::from_value::<CompletionParams>(request.params) {
        Err(err) => {
            return Some(get_response_error(
                request.id,
                ErrorCode::InvalidParams,
                format!("Could not parse completion params: {:?}", err),
            ));
        }
        Ok(value) => value,
    };

    let mut position = params.text_document_position.position;
    // We're interested in the char before the cursor.
    if position.character > 0 {
        position.character -= 1;
    }

    let uri = &params.text_document_position.text_document.uri.to_string();
    let mut token: Option<Token> = None;
    if let Some(document) = DOCUMENT_STORE.lock().unwrap().get_document(uri) {
        token = document.get_token_under_cursor(position);
    }

    let mut completion_items: Vec<CompletionItem> = get_global_snippets();

    if let Some(token) = token {
        if let TokenData::DrupalRouteReference(_) = token.data {
            DOCUMENT_STORE
                .lock()
                .unwrap()
                .get_documents()
                .values()
                .for_each(|document| {
                    document.tokens.iter().for_each(|token| {
                        if let TokenData::DrupalRouteDefinition(route) = &token.data {
                            let mut documentation = None;
                            if let Some(documentation_string) = get_documentation_for_token(token) {
                                documentation = Some(Documentation::String(documentation_string));
                            }
                            completion_items.push(CompletionItem {
                                label: route.name.clone(),
                                kind: Some(CompletionItemKind::REFERENCE),
                                documentation,
                                deprecated: Some(false),
                                ..CompletionItem::default()
                            });
                        }
                    })
                });
        }
        if let TokenData::DrupalServiceReference(_) = token.data {
            DOCUMENT_STORE
                .lock()
                .unwrap()
                .get_documents()
                .values()
                .for_each(|document| {
                    document.tokens.iter().for_each(|token| {
                        if let TokenData::DrupalServiceDefinition(service) = &token.data {
                            let mut documentation = None;
                            if let Some(documentation_string) = get_documentation_for_token(token) {
                                documentation = Some(Documentation::String(documentation_string));
                            }
                            completion_items.push(CompletionItem {
                                label: service.name.clone(),
                                kind: Some(CompletionItemKind::REFERENCE),
                                documentation,
                                deprecated: Some(false),
                                ..CompletionItem::default()
                            });
                        }
                    })
                });
        }
    }

    let (file_name, extension) = uri.split('/').last()?.split_once('.')?;
    if extension == "module" || extension == "theme" {
        DOCUMENT_STORE
            .lock()
            .unwrap()
            .get_documents()
            .values()
            .for_each(|document| {
                document.tokens.iter().for_each(|token| {
                    if let TokenData::DrupalHookDefinition(hook) = &token.data {
                        let mut documentation = None;
                        if let Some(documentation_string) = get_documentation_for_token(token) {
                            documentation = Some(Documentation::String(documentation_string));
                        }
                        // Regex to replace placeholders in hook names.
                        let re = Regex::new(r"([A-Z][A-Z_]+[A-Z])").unwrap();
                        completion_items.push(CompletionItem {
                            label: hook.name.clone(),
                            label_details: Some(CompletionItemLabelDetails {
                                description: Some("hook".to_string()),
                                detail: None,
                            }),
                            kind: Some(CompletionItemKind::SNIPPET),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            insert_text: Some(
                                format!(
                                    "/**\n * Implements {}().\n */\nfunction {}_{}({}) {{\n  $0\n}}",
                                    hook.name,
                                    file_name,
                                    re.replace_all(hook.name.replace("hook_", "").as_str(), r"$${$1}"),
                                    hook.parameters.clone().unwrap_or("".to_string()).replace("$", "\\$")
                                )
                                .to_string(),
                            ),
                            documentation,
                            deprecated: Some(false),
                            ..CompletionItem::default()
                        });
                    }
                })
            });
    }

    if completion_items.len() == 0 {
        return Some(Response {
            id: request.id,
            result: None,
            error: None,
        });
    }

    let completion_result = CompletionList {
        is_incomplete: false,
        items: completion_items,
    };

    return match serde_json::to_value(completion_result) {
        Ok(result) => Some(Response {
            id: request.id,
            result: Some(result),
            error: None,
        }),
        Err(error) => Some(get_response_error(
            request.id,
            ErrorCode::InternalError,
            format!("Unable to parse completion result: {:?}", error),
        )),
    };
}

fn get_global_snippets() -> Vec<CompletionItem> {
    let mut snippets = HashMap::new();

    snippets.insert(
        "batch",
        r#"
\$storage = \\Drupal::entityTypeManager()->getStorage('$0');
if (!isset(\$sandbox['ids'])) {
  \$ids = \$storage->getQuery()
    ->accessCheck(FALSE)
    ->execute();
  \$sandbox['ids'] = \$ids;
  \$sandbox['total'] = count(\$sandbox['ids']);
}

\$ids = array_splice(\$sandbox['ids'], 0, 20);
foreach (\$storage->loadMultiple(\$ids) as \$entity) {
  \$entity->save();
}

if (\$sandbox['total'] > 0) {
  \$sandbox['#finished'] = (\$sandbox['total'] - count(\$sandbox['ids'])) / \$sandbox['total'];
}"#,
    );
    snippets.insert(
        "ihdoc",
        r#"
/**
 * {@inheritdoc}
 */"#,
    );
    snippets.insert(
        "entity-storage",
        "\\$storage = \\$this->entityTypeManager->getStorage('$0');",
    );
    snippets.insert(
        "entity-load",
        "\\$$1 = \\$this->entityTypeManager->getStorage('$1')->load($0);",
    );

    snippets
        .iter()
        .map(|(name, snippet)| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            deprecated: Some(false),
            ..CompletionItem::default()
        })
        .collect()
}
