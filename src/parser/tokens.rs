use regex::Regex;
use std::{collections::HashMap, fmt};
use tree_sitter::Range;

use crate::document_store::DocumentStore;

#[derive(Debug)]
pub struct Token {
    pub range: Range,
    pub data: TokenData,
}

impl Token {
    pub fn new(data: TokenData, range: Range) -> Self {
        Self { data, range }
    }
}

#[derive(Debug)]
pub enum TokenData {
    PhpClassReference(PhpClassName),
    PhpClassDefinition(PhpClass),
    PhpMethodReference(PhpMethod),
    PhpMethodDefinition(PhpMethod),
    DrupalRouteReference(String),
    DrupalRouteDefinition(DrupalRoute),
    DrupalServiceReference(String),
    DrupalServiceDefinition(DrupalService),
    DrupalHookReference(String),
    DrupalHookDefinition(DrupalHook),
    DrupalPermissionDefinition(DrupalPermission),
    DrupalPermissionReference(String),
    DrupalPluginReference(DrupalPluginReference),
    DrupalTranslationString(DrupalTranslationString),
}

#[derive(Debug, PartialEq, Clone)]
pub struct PhpClassName {
    value: String,
}

impl fmt::Display for PhpClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<&str> for PhpClassName {
    fn from(value: &str) -> Self {
        Self {
            // Trim single quotes and backslashes to ensure the most trimmed down version of a
            // fully qualified class name.
            value: value.trim_matches(['\'', '\\']).to_string(),
        }
    }
}

#[derive(Debug)]
pub enum ClassAttribute {
    Plugin(DrupalPlugin),
}

#[derive(Debug)]
pub struct PhpClass {
    pub name: PhpClassName,
    pub attribute: Option<ClassAttribute>,
    pub methods: HashMap<String, Box<Token>>,
}

#[derive(Debug)]
pub struct PhpMethod {
    pub name: String,
    pub class_name: Option<PhpClassName>,
    pub service_name: Option<String>,
}

impl PhpMethod {
    pub fn get_class(&self, store: &DocumentStore) -> Option<PhpClassName> {
        if let Some(class_name) = &self.class_name {
            return Some(class_name.clone());
        } else if let Some(service_name) = &self.service_name {
            if let Some((_, token)) = store.get_service_definition(service_name) {
                if let TokenData::DrupalServiceDefinition(service) = &token.data {
                    return Some(service.class.clone());
                }
            }
        }
        None
    }
}

impl TryFrom<&str> for PhpMethod {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some((class, method)) = value.trim_matches(['\'', '\\']).split_once("::") {
            return Ok(Self {
                name: method.to_string(),
                class_name: Some(PhpClassName::from(class)),
                service_name: None,
            });
        }

        Err("Unable to convert string to PhpMethod")
    }
}

#[derive(Debug)]
pub struct DrupalRoute {
    pub name: String,
    pub path: String,
    pub _defaults: DrupalRouteDefaults,
}

impl DrupalRoute {
    pub fn get_route_parameters(&self) -> Vec<&str> {
        let re = Regex::new(r"\{([^{}]+)\}");
        match re {
            Ok(re) => re
                .captures_iter(&self.path)
                .map(|c| c.get(1).unwrap().as_str())
                .collect(),
            Err(_) => vec![],
        }
    }
}

#[derive(Debug)]
pub struct DrupalRouteDefaults {
    pub _controller: Option<PhpMethod>,
    pub _form: Option<PhpClassName>,
    pub _entity_form: Option<String>,
    pub _title: Option<String>,
}

#[derive(Debug)]
pub struct DrupalService {
    pub name: String,
    pub class: PhpClassName,
}

#[derive(Debug)]
pub struct DrupalHook {
    pub name: String,
    pub parameters: Option<String>,
}

#[derive(Debug)]
pub struct DrupalPermission {
    pub name: String,
    pub title: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DrupalPluginType {
    EntityType,
    QueueWorker,
    FieldType,
    DataType,
    FormElement,
    RenderElement,
}

impl TryFrom<&str> for DrupalPluginType {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ContentEntityType" | "ConfigEntityType" => Ok(DrupalPluginType::EntityType),
            "QueueWorker" => Ok(DrupalPluginType::QueueWorker),
            "FieldType" => Ok(DrupalPluginType::FieldType),
            "DataType" => Ok(DrupalPluginType::DataType),
            "FormElement" => Ok(DrupalPluginType::FormElement),
            "RenderElement" => Ok(DrupalPluginType::RenderElement),
            _ => Err("Unable to convert string to DrupalPluginType"),
        }
    }
}

impl fmt::Display for DrupalPluginType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct DrupalPlugin {
    pub plugin_type: DrupalPluginType,
    pub plugin_id: String,
    pub usage_example: Option<String>,
}

#[derive(Debug)]
pub struct DrupalPluginReference {
    pub plugin_type: DrupalPluginType,
    pub plugin_id: String,
}

#[derive(Debug)]
pub struct DrupalTranslationString {
    pub string: String,
    pub placeholders: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_php_class_from_string() {
        assert_eq!(
            "Drupal\\test\\TestClass",
            PhpClassName::from("Drupal\\test\\TestClass").to_string()
        );
        assert_eq!(
            "Drupal\\test\\TestClass",
            PhpClassName::from("\\Drupal\\test\\TestClass").to_string()
        );
        assert_eq!(
            "Drupal\\test\\TestClass",
            PhpClassName::from("'\\Drupal\\test\\TestClass\\'").to_string()
        );
    }

    #[test]
    fn create_php_method_from_string() {
        assert_eq!(
            "myMethod",
            PhpMethod::try_from("Drupal\\test\\TestClass::myMethod")
                .unwrap()
                .name
        );
        assert_eq!(
            "Drupal\\test\\TestClass",
            PhpMethod::try_from("Drupal\\test\\TestClass::myMethod")
                .unwrap()
                .class_name
                .unwrap()
                .to_string()
        );
        assert_eq!(
            "myMethod",
            PhpMethod::try_from("'\\Drupal\\test\\TestClass::myMethod'")
                .unwrap()
                .name
        );
        assert_eq!(
            "Drupal\\test\\TestClass",
            PhpMethod::try_from("'\\Drupal\\test\\TestClass::myMethod'")
                .unwrap()
                .class_name
                .unwrap()
                .to_string()
        );
        assert!(PhpMethod::try_from("invalid class").is_err());
    }
}
