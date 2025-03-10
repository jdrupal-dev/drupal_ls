use regex::Regex;
use std::collections::HashMap;
use tree_sitter::Range;

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
}

#[derive(Debug, PartialEq)]
pub struct PhpClassName {
    value: String,
}

impl std::fmt::Display for PhpClassName {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
pub struct PhpClass {
    pub name: PhpClassName,
    pub methods: HashMap<String, Box<Token>>,
}

#[derive(Debug)]
pub struct PhpMethod {
    pub name: String,
    pub class_name: PhpClassName,
}

impl TryFrom<&str> for PhpMethod {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some((class, method)) = value.trim_matches(['\'', '\\']).split_once("::") {
            return Ok(Self {
                name: method.to_string(),
                class_name: PhpClassName::from(class),
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
                .to_string()
        );
        assert!(PhpMethod::try_from("invalid class").is_err());
    }
}
