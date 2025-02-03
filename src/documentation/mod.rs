use crate::document_store::DOCUMENT_STORE;
use crate::parser::tokens::*;

const CLASS_REFERENCE: &str = r#"
# Class reference

@see [@class_name](@class_name)
"#;

const SERVICE_REFERENCE: &str = r#"
# Service reference: @name

*Implementation:*
```yaml
@definition
```

@see [@uri](@uri)
"#;

const SERVICE_DEFINITION: &str = r#"
# Service: @name

*Class:* @class
"#;

const ROUTE_REFERENCE: &str = r#"
# Route reference: @name

*Implementation:*
```yaml
@definition
```

@see [@uri](@uri)
"#;

const ROUTE_DEFINITION: &str = r#"
# Route: @name

*Path:* @path
"#;

const HOOK_REFERENCE: &str = r#"
# Hook reference: @name

*Implementation:*
```yaml
@definition
```

@see [@uri](@uri)
"#;

const HOOK_DEFINITION: &str = r#"
# Hook: @name

```php
<?php function @name(@parameters) {}
```
"#;

const PERMISSION_REFERENCE: &str = r#"
# Permission reference: @name

*Implementation:*
```yaml
@definition
```

@see [@uri](@uri)
"#;

const PERMISSION_DEFINITION: &str = r#"
# Permission: @name
Title: @title
"#;

pub fn get_documentation_for_token(token: &Token) -> Option<String> {
    match &token.data {
        TokenData::PhpClassReference(class) => {
            Some(CLASS_REFERENCE.replace("@class_name", &class.to_string()))
        }
        TokenData::PhpMethodReference(method) => Some(format!(
            "PHP Method reference\nclass: {}\nmethod: {}",
            method.class_name.to_string(),
            method.name
        )),
        TokenData::DrupalRouteReference(route_name) => {
            let store = DOCUMENT_STORE.lock().unwrap();

            let (source_document, token) = store.get_route_definition(route_name)?;
            if let TokenData::DrupalRouteDefinition(route) = &token.data {
                let definition =
                    &source_document.content[token.range.start_byte..token.range.end_byte];

                return Some(
                    ROUTE_REFERENCE
                        .replace("@name", route.name.as_str())
                        .replace("@uri", source_document.get_uri()?.as_str())
                        .replace("@definition", definition),
                );
            }
            None
        }
        TokenData::DrupalRouteDefinition(route) => Some(
            ROUTE_DEFINITION
                .replace("@name", &route.name)
                .replace("@path", &route.path),
        ),
        TokenData::DrupalServiceReference(service_name) => {
            let store = DOCUMENT_STORE.lock().unwrap();

            let (source_document, token) = store.get_service_definition(service_name)?;
            if let TokenData::DrupalServiceDefinition(service) = &token.data {
                let definition =
                    &source_document.content[token.range.start_byte..token.range.end_byte];

                return Some(
                    SERVICE_REFERENCE
                        .replace("@name", service.name.as_str())
                        .replace("@uri", source_document.get_uri()?.as_str())
                        .replace("@definition", definition),
                );
            }
            None
        }
        TokenData::DrupalServiceDefinition(service) => Some(
            SERVICE_DEFINITION
                .replace("@name", &service.name)
                .replace("@class", &service.class.to_string()),
        ),
        TokenData::DrupalHookReference(hook_name) => {
            let store = DOCUMENT_STORE.lock().unwrap();

            let (source_document, token) = store.get_hook_definition(hook_name)?;
            if let TokenData::DrupalHookDefinition(hook) = &token.data {
                let definition =
                    &source_document.content[token.range.start_byte..token.range.end_byte];

                return Some(
                    HOOK_REFERENCE
                        .replace("@name", hook.name.as_str())
                        .replace("@uri", source_document.get_uri()?.as_str())
                        .replace("@definition", definition),
                );
            }
            None
        }
        TokenData::DrupalHookDefinition(hook) => {
            Some(HOOK_DEFINITION.replace("@name", &hook.name).replace(
                "@parameters",
                &hook.parameters.clone().unwrap_or(String::default()),
            ))
        }
        TokenData::DrupalPermissionReference(permission_name) => {
            let store = DOCUMENT_STORE.lock().unwrap();

            let (source_document, token) = store.get_permission_definition(permission_name)?;
            if let TokenData::DrupalPermissionDefinition(permission) = &token.data {
                let definition =
                    &source_document.content[token.range.start_byte..token.range.end_byte];

                return Some(
                    PERMISSION_REFERENCE
                        .replace("@name", &permission.name)
                        .replace("@uri", source_document.get_uri()?.as_str())
                        .replace("@definition", definition),
                );
            }
            None
        }
        TokenData::DrupalPermissionDefinition(permission) => Some(
            PERMISSION_DEFINITION
                .replace("@name", &permission.name)
                .replace("@title", &permission.title),
        ),
        _ => None,
    }
}
