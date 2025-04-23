use std::collections::HashMap;

use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionList,
    CompletionParams, CompletionTextEdit, Documentation, InsertTextFormat, Position, Range,
    TextEdit,
};
use regex::Regex;

use crate::document_store::DOCUMENT_STORE;
use crate::documentation::get_documentation_for_token;
use crate::parser::tokens::{ClassAttribute, DrupalPluginType, Token, TokenData};
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
    let mut current_line: String = String::default();
    if let Some(document) = DOCUMENT_STORE.lock().unwrap().get_document(uri) {
        current_line = document
            .content
            .lines()
            .nth(params.text_document_position.position.line as usize)
            .unwrap_or("")
            .to_string();
        token = document.get_token_under_cursor(position);
    }

    let (file_name, extension) = uri.split('/').last()?.split_once('.')?;

    let mut completion_items: Vec<CompletionItem> = get_global_snippets();
    if let Some(token) = token {
        if let TokenData::DrupalRouteReference(_) = token.data {
            let re = Regex::new(r"(?<method>.*fromRoute\(')(?<name>[^']*)'(?<params>, \[.*\])?");
            let mut method_len = 0;
            let mut name_len = 0;
            let mut params_len = 0;
            if let Some(captures) = re.unwrap().captures(current_line.as_str()) {
                method_len = match captures.name("method") {
                    Some(str) => str.len() as u32,
                    None => 0,
                };
                // TODO: name_len is sometimes incorrect if typing too fast.
                name_len = match captures.name("name") {
                    Some(str) => str.len() as u32,
                    None => 0,
                };
                params_len = match captures.name("params") {
                    Some(str) => str.len() as u32,
                    None => 0,
                };
            }

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

                            let mut text_edit = None;
                            let mut additional_text_edits = None;
                            if method_len > 0 {
                                text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                                    range: Range {
                                        start: Position {
                                            line: params.text_document_position.position.line,
                                            character: method_len,
                                        },
                                        end: Position {
                                            line: params.text_document_position.position.line,
                                            character: params
                                                .text_document_position
                                                .position
                                                .character,
                                        },
                                    },
                                    new_text: route.name.to_string(),
                                }));

                                let route_parameters = route.get_route_parameters();
                                let mut route_parameters_text = String::new();
                                if !route_parameters.is_empty() {
                                    route_parameters_text = format!(
                                        ", [{}]",
                                        route_parameters
                                            .iter()
                                            .map(|&p| format!("'{}' => ${}", p, p))
                                            .collect::<Vec<String>>()
                                            .join(", ")
                                    );
                                }

                                additional_text_edits = Some(vec![TextEdit {
                                    range: Range {
                                        start: Position {
                                            line: params.text_document_position.position.line,
                                            character: method_len + name_len + 1,
                                        },
                                        end: Position {
                                            line: params.text_document_position.position.line,
                                            character: method_len + name_len + 1 + params_len,
                                        },
                                    },
                                    new_text: route_parameters_text,
                                }]);
                            }
                            completion_items.push(CompletionItem {
                                label: route.name.clone(),
                                label_details: Some(CompletionItemLabelDetails {
                                    description: Some("Route".to_string()),
                                    detail: None,
                                }),
                                kind: Some(CompletionItemKind::REFERENCE),
                                documentation,
                                text_edit,
                                additional_text_edits,
                                deprecated: Some(false),
                                ..CompletionItem::default()
                            });
                        }
                    })
                });
        } else if let TokenData::DrupalServiceReference(_) = token.data {
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
                                label_details: Some(CompletionItemLabelDetails {
                                    description: Some("Service".to_string()),
                                    detail: None,
                                }),
                                kind: Some(CompletionItemKind::REFERENCE),
                                documentation,
                                deprecated: Some(false),
                                ..CompletionItem::default()
                            });
                        }
                    })
                });
        } else if let TokenData::PhpMethodReference(method) = token.data {
            let store = DOCUMENT_STORE.lock().unwrap();
            // TODO: Don't suggest private/protected methods.
            if let Some((_, class_token)) = store.get_class_definition(&method.get_class(&store)?) {
                if let TokenData::PhpClassDefinition(class) = &class_token.data {
                    class.methods.keys().for_each(|method_name| {
                        completion_items.push(CompletionItem {
                            label: method_name.clone(),
                            label_details: Some(CompletionItemLabelDetails {
                                description: Some("Method".to_string()),
                                detail: None,
                            }),
                            kind: Some(CompletionItemKind::REFERENCE),
                            documentation: None,
                            deprecated: Some(false),
                            ..CompletionItem::default()
                        });
                    });
                }
            }
        } else if let TokenData::DrupalPermissionReference(_) = token.data {
            DOCUMENT_STORE
                .lock()
                .unwrap()
                .get_documents()
                .values()
                .for_each(|document| {
                    document.tokens.iter().for_each(|token| {
                        if let TokenData::DrupalPermissionDefinition(permission) = &token.data {
                            let mut documentation = None;
                            if let Some(documentation_string) = get_documentation_for_token(token) {
                                documentation = Some(Documentation::String(documentation_string));
                            }
                            // TODO: Figure out how to correctly deal with whitespaces in the
                            // label.
                            completion_items.push(CompletionItem {
                                label: permission.name.clone(),
                                label_details: Some(CompletionItemLabelDetails {
                                    description: Some("Permission".to_string()),
                                    detail: None,
                                }),
                                kind: Some(CompletionItemKind::REFERENCE),
                                documentation,
                                deprecated: Some(false),
                                ..CompletionItem::default()
                            });
                        }
                    })
                });
        } else if let TokenData::DrupalPluginReference(plugin_reference) = token.data {
            DOCUMENT_STORE
                .lock()
                .unwrap()
                .get_documents()
                .values()
                .for_each(|document| {
                    document.tokens.iter().for_each(|token| {
                        if let TokenData::PhpClassDefinition(class) = &token.data {
                            if let Some(ClassAttribute::Plugin(plugin)) = &class.attribute {
                                if plugin_reference.plugin_type == plugin.plugin_type {
                                    let mut documentation = None;
                                    if let Some(documentation_string) =
                                        get_documentation_for_token(token)
                                    {
                                        documentation =
                                            Some(Documentation::String(documentation_string));
                                    }
                                    completion_items.push(CompletionItem {
                                        label: plugin.plugin_id.clone(),
                                        label_details: Some(CompletionItemLabelDetails {
                                            description: Some(plugin.plugin_type.to_string()),
                                            detail: None,
                                        }),
                                        kind: Some(CompletionItemKind::REFERENCE),
                                        documentation,
                                        deprecated: Some(false),
                                        ..CompletionItem::default()
                                    });
                                }
                            }
                        }
                    })
                });
        }
    } else if extension == "module" || extension == "theme" {
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

    if completion_items.is_empty() {
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

    match serde_json::to_value(completion_result) {
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
    }
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
        "ensure-instanceof",
        "if (!($1 instanceof $2)) {\n  return$0;\n}",
    );
    snippets.insert(
        "entity-storage",
        "\\$storage = \\$this->entityTypeManager->getStorage('$0');",
    );
    snippets.insert(
        "entity-load",
        "\\$$1 = \\$this->entityTypeManager->getStorage('$1')->load($0);",
    );
    snippets.insert(
        "entity-query",
        r#"
\$ids = \$this->entityTypeManager->getStorage('$1')->getQuery()
  ->accessCheck(${TRUE})
  $0
  ->execute()"#,
    );
    snippets.insert("type", "'#type' => '$0',");
    snippets.insert("title", "'#title' => \\$this->t('$0'),");
    snippets.insert("description", "'#description' => \\$this->t('$0'),");
    snippets.insert("attributes", "'#attributes' => [$0],");
    snippets.insert(
        "attributes-class",
        "'#attributes' => [\n  'class' => ['$0'],\n],",
    );
    snippets.insert("attributes-id", "'#attributes' => [\n  'id' => '$0',\n],");
    snippets.insert(
        "type_html_tag",
        r#"'#type' => 'html_tag',
'#tag' => '$1',
'#value' => $0,"#,
    );
    snippets.insert(
        "type_details",
        r#"'#type' => 'details',
'#open' => TRUE,
'#title' => \$this->t('$0'),"#,
    );
    snippets.insert(
        "create",
        r#"/**
 * {@inheritdoc}
 */
public static function create(ContainerInterface \$container) {
  return new static(
    \$container->get('$0'),
  );
}"#,
    );
    snippets.insert(
        "create-plugin",
        r#"/**
 * {@inheritdoc}
 */
public static function create(ContainerInterface \$container, array \$configuration, \$plugin_id, \$plugin_definition) {
  return new static(
    \$configuration,
    \$plugin_id,
    \$plugin_definition,
    \$container->get('$0'),
  );
}"#,
    );

    // Create pre-generated snippets.
    DOCUMENT_STORE
        .lock()
        .unwrap()
        .get_documents()
        .values()
        .flat_map(|document| document.tokens.iter())
        .filter_map(|token| match &token.data {
            TokenData::PhpClassDefinition(class_def) => match &class_def.attribute {
                Some(ClassAttribute::Plugin(plugin)) => Some(plugin),
                _ => None,
            },
            _ => None,
        })
        .filter_map(|plugin| {
            let snippet_key_prefix = match plugin.plugin_type {
                DrupalPluginType::RenderElement => Some("render"),
                DrupalPluginType::FormElement => Some("form"),
                _ => None,
            };

            snippet_key_prefix.and_then(|prefix| {
                plugin
                    .usage_example
                    .as_ref()
                    .map(|usage_example| (prefix, &plugin.plugin_id, usage_example))
            })
        })
        .for_each(|(snippet_key_prefix, plugin_id, usage_example)| {
            let key_string: String = format!("{}-{}", snippet_key_prefix, plugin_id);
            let value_string: String = usage_example.replace("$", "\\$");

            let key: &'static str = Box::leak(key_string.into_boxed_str());
            let value: &'static str = Box::leak(value_string.into_boxed_str());
            snippets.insert(key, value);
        });

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
