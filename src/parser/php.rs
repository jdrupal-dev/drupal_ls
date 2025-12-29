use lsp_types::Position;
use regex::Regex;
use std::collections::HashMap;
use tree_sitter::{Node, Point};

use super::tokens::{
    ClassAttribute, DrupalHook, DrupalPlugin, DrupalPluginReference, DrupalPluginType, DrupalTranslationString, PhpClass, PhpClassName, PhpMethod, Token, TokenData
};
use super::{get_closest_parent_by_kind, get_node_at_position, get_tree, position_to_point};

pub struct PhpParser {
    source: String,
}

impl PhpParser {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
        }
    }

    pub fn get_tokens(&self) -> Vec<Token> {
        let tree = get_tree(&self.source, &tree_sitter_php::LANGUAGE_PHP.into());
        self.parse_nodes(vec![tree.unwrap().root_node()])
    }

    pub fn get_token_at_position(&self, position: Position) -> Option<Token> {
        let tree = get_tree(&self.source, &tree_sitter_php::LANGUAGE_PHP.into())?;
        let mut node = get_node_at_position(&tree, position)?;
        let point = position_to_point(position);

        // Return the first "parseable" token in the parent chain.
        let mut parsed_node: Option<Token>;
        loop {
            parsed_node = self.parse_node(node, Some(point));
            if parsed_node.is_some() {
                break;
            }
            node = node.parent()?;
        }
        parsed_node
    }

    fn parse_nodes(&self, nodes: Vec<Node>) -> Vec<Token> {
        let mut tokens: Vec<Token> = vec![];

        let mut current_nodes: Box<Vec<Node>> = Box::new(nodes);
        while current_nodes.len() > 0 {
            let mut new_nodes: Box<Vec<Node>> = Box::default();
            for node in current_nodes.into_iter() {
                if node.is_error() {
                    continue;
                }

                match self.parse_node(node, None) {
                    Some(token) => tokens.push(token),
                    None => {
                        if node.child_count() > 0 {
                            let mut cursor = node.walk();
                            new_nodes
                                .append(&mut node.children(&mut cursor).collect::<Vec<Node>>());
                        }
                    }
                };
            }
            current_nodes = new_nodes;
        }
        tokens
    }

    fn parse_node(&self, node: Node, point: Option<Point>) -> Option<Token> {
        match node.kind() {
            "class_declaration" => self.parse_class_declaration(node),
            "method_declaration" => self.parse_method_declaration(node),
            "scoped_call_expression" | "member_call_expression" | "function_call_expression" => {
                self.parse_call_expression(node, point)
            }
            "function_definition" => self.parse_function_definition(node),
            "comment" => self.parse_comment(node),
            _ => None,
        }
    }

    fn parse_function_definition(&self, node: Node) -> Option<Token> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(&name_node);
        if name.starts_with("hook") {
            let parameters_node = node.child_by_field_name("parameters")?;
            let parameters = self
                .get_node_text(&parameters_node)
                .trim_matches(['(', ')']);

            return Some(Token::new(
                TokenData::DrupalHookDefinition(DrupalHook {
                    name: name.to_string(),
                    parameters: Some(parameters.to_string()),
                }),
                node.range(),
            ));
        }
        None
    }

    fn parse_comment(&self, node: Node) -> Option<Token> {
        let text = self.get_node_text(&node);

        // A comment with the text "Implements hook_NAME" is a reference to a Drupal hook.
        if text.contains("Implements hook_") {
            let start_bytes = text.find("hook_")?;
            let end_bytes = text.find("()")?;

            let hook_name = &text[start_bytes..end_bytes];
            return Some(Token::new(
                TokenData::DrupalHookReference(hook_name.to_string()),
                node.range(),
            ));
        }

        None
    }

    fn parse_call_expression(&self, node: Node, point: Option<Point>) -> Option<Token> {
        let string_content = node.descendant_for_point_range(point?, point?)?;
        let name_node = match node.kind() {
            "function_call_expression" => node.child_by_field_name("function"),
            _ => node.child_by_field_name("name"),
        }?;

        let name = self.get_node_text(&name_node);

        if node.kind() == "member_call_expression" {
            let object_node = node.child_by_field_name("object")?;
            if self.get_node_text(&object_node).contains("Drupal::service") {
                let arguments = object_node.child_by_field_name("arguments")?;
                let service_name = self
                    .get_node_text(&arguments)
                    .trim_matches(|c| c == '\'' || c == '(' || c == ')');
                return Some(Token::new(
                    TokenData::PhpMethodReference(PhpMethod {
                        name: name.to_string(),
                        class_name: None,
                        service_name: Some(service_name.to_string()),
                    }),
                    node.range(),
                ));
            }
        }

        if string_content.kind() != "string_content" {
            return None;
        }

        if name == "fromRoute" || name == "createFromRoute" || name == "setRedirect" {
            return Some(Token::new(
                TokenData::DrupalRouteReference(self.get_node_text(&string_content).to_string()),
                node.range(),
            ));
        } else if name == "service" {
            return Some(Token::new(
                TokenData::DrupalServiceReference(self.get_node_text(&string_content).to_string()),
                node.range(),
            ));
        } else if name == "hasPermission" {
            return Some(Token::new(
                TokenData::DrupalPermissionReference(
                    self.get_node_text(&string_content).to_string(),
                ),
                node.range(),
            ));
        }
        // TODO: This is a quite primitive way to detect ContainerInterface::get.
        // Can we somehow get the interface of a given variable?
        else if name == "get" {
            let object_node = node.child_by_field_name("object")?;
            let object = self.get_node_text(&object_node);
            if object == "$container" {
                return Some(Token::new(
                    TokenData::DrupalServiceReference(
                        self.get_node_text(&string_content).to_string(),
                    ),
                    node.range(),
                ));
            } else if object.contains("queueFactory") {
                return Some(Token::new(
                    TokenData::DrupalPluginReference(DrupalPluginReference {
                        plugin_type: DrupalPluginType::QueueWorker,
                        plugin_id: self.get_node_text(&string_content).to_string(),
                    }),
                    node.range(),
                ));
            }
        } else if name == "getStorage" {
            let object_node = node.child_by_field_name("object")?;
            let object = self.get_node_text(&object_node);
            if object.contains("entityTypeManager") {
                return Some(Token::new(
                    TokenData::DrupalPluginReference(DrupalPluginReference {
                        plugin_type: DrupalPluginType::EntityType,
                        plugin_id: self.get_node_text(&string_content).to_string(),
                    }),
                    node.range(),
                ));
            }
        } else if name == "create" {
            let scope_node = node.child_by_field_name("scope")?;
            if self
                .get_node_text(&scope_node)
                .contains("BaseFieldDefinition")
            {
                return Some(Token::new(
                    TokenData::DrupalPluginReference(DrupalPluginReference {
                        plugin_type: DrupalPluginType::FieldType,
                        plugin_id: self.get_node_text(&string_content).to_string(),
                    }),
                    node.range(),
                ));
            } else if self.get_node_text(&scope_node).contains("DataDefinition") {
                return Some(Token::new(
                    TokenData::DrupalPluginReference(DrupalPluginReference {
                        plugin_type: DrupalPluginType::DataType,
                        plugin_id: self.get_node_text(&string_content).to_string(),
                    }),
                    node.range(),
                ));
            }
        } else if name == "queue" {
            return Some(Token::new(
                TokenData::DrupalPluginReference(DrupalPluginReference {
                    plugin_type: DrupalPluginType::QueueWorker,
                    plugin_id: self.get_node_text(&string_content).to_string(),
                }),
                node.range(),
            ));
        } else if name == "t" {
            return Some(Token::new(
                TokenData::DrupalTranslationString(DrupalTranslationString {
                    string: self.get_node_text(&string_content).to_string(),
                    placeholders: None,
                }),
                node.range(),
            ));
        }

        None
    }

    fn parse_class_declaration(&self, node: Node) -> Option<Token> {
        let mut methods: HashMap<String, Box<Token>> = HashMap::new();
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            body_node.children(&mut cursor).for_each(|child| {
                if let Some(token) = self.parse_method_declaration(child) {
                    if let TokenData::PhpMethodDefinition(token_data) = &token.data {
                        methods.insert(token_data.name.clone(), Box::new(token));
                    }
                }
            });
        }

        let mut class_attribute = None;
        if let Some(attributes_node) = node.child_by_field_name("attributes") {
            let attribute_group = attributes_node.child(0)?;
            class_attribute = self.parse_class_attribute(attribute_group.named_child(0)?);
        } else if let Some(comment_node) = node.prev_named_sibling() {
            if comment_node.kind() == "comment" {
                let text = self.get_node_text(&comment_node);

                let re = Regex::new(r#"\*\s*@(?<type>.+)\("#).unwrap();
                let mut plugin_type: Option<DrupalPluginType> = None;
                if let Some(captures) = re.captures(text) {
                    if let Some(str) = captures.name("type") {
                        plugin_type = DrupalPluginType::try_from(str.as_str()).ok();
                    }
                }

                let re = Regex::new(r#"id\s*=\s*"(?<id>[^"]+)""#).unwrap();
                let mut plugin_id: Option<String> = None;
                if let Some(captures) = re.captures(text) {
                    if let Some(str) = captures.name("id") {
                        plugin_id = Some(str.as_str().to_string());
                    }
                }

                if let (Some(plugin_type), Some(plugin_id)) = (plugin_type, plugin_id) {
                    class_attribute = Some(ClassAttribute::Plugin(DrupalPlugin {
                        plugin_type,
                        plugin_id,
                        usage_example: self.extract_usage_example_from_comment(&comment_node),
                    }));
                };
            }
        }

        Some(Token::new(
            TokenData::PhpClassDefinition(PhpClass {
                name: self.get_class_name_from_node(node)?,
                attribute: class_attribute,
                methods,
            }),
            node.range(),
        ))
    }

    fn parse_method_declaration(&self, node: Node) -> Option<Token> {
        if node.kind() != "method_declaration" {
            return None;
        }

        let class_node = get_closest_parent_by_kind(&node, "class_declaration")?;

        let name_node = node.child_by_field_name("name")?;
        Some(Token::new(
            TokenData::PhpMethodDefinition(PhpMethod {
                name: self.get_node_text(&name_node).to_string(),
                class_name: self.get_class_name_from_node(class_node),
                service_name: None,
            }),
            node.range(),
        ))
    }

    fn parse_class_attribute(&self, node: Node) -> Option<ClassAttribute> {
        if node.kind() != "attribute" {
            return None;
        }

        let mut plugin_id = String::default();

        // TODO: Look into improving this if we want to extract more than plugin id.
        let parameters_node = node.child_by_field_name("parameters")?;
        for argument in parameters_node.named_children(&mut parameters_node.walk()) {
            // In the case of f.e `#[FormElement('date')]` there is no `id` field.
            if self.get_node_text(&argument).starts_with("'")
                && self.get_node_text(&argument).ends_with("'")
            {
                plugin_id = self
                    .get_node_text(&argument)
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
                break;
            }
            let argument_name = argument.child_by_field_name("name")?;
            if self.get_node_text(&argument_name) == "id" {
                plugin_id = self
                    .get_node_text(&argument.named_child(1)?)
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string()
            }
        }

        match DrupalPluginType::try_from(self.get_node_text(&node.child(0)?)) {
            Ok(plugin_type) => Some(ClassAttribute::Plugin(DrupalPlugin {
                plugin_id,
                plugin_type,
                usage_example: self.extract_usage_example_from_comment(
                    &node.parent()?.parent()?.parent()?.prev_named_sibling()?,
                ),
            })),
            Err(_) => None,
        }
    }

    fn get_class_name_from_node(&self, node: Node) -> Option<PhpClassName> {
        if node.kind() != "class_declaration" {
            return None;
        }
        let mut prev = node.prev_sibling();
        while prev?.kind() != "namespace_definition" {
            prev = prev?.prev_sibling();
        }

        if prev?.kind() != "namespace_definition" {
            return None;
        }

        let namespace_node = prev?.child_by_field_name("name");
        let namespace = self.get_node_text(&namespace_node?);
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(&name_node);
        Some(PhpClassName::from(
            format!("{}\\{}", namespace, name).as_str(),
        ))
    }

    fn get_node_text(&self, node: &Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Helper function to extract usage example from the preceding comment.
    fn extract_usage_example_from_comment(&self, comment_node: &Node) -> Option<String> {
        if comment_node.kind() != "comment" {
            return None;
        }

        let comment_text = self.get_node_text(comment_node);
        let start_tag = "@code";
        let end_tag = "@endcode";

        if let (Some(start_index), Some(end_index)) =
            (comment_text.find(start_tag), comment_text.find(end_tag))
        {
            if end_index > start_index {
                let code_start = start_index + start_tag.len();
                let example = comment_text[code_start..end_index].trim();

                // Regex to replace "* " or "*" from the beginning of a line.
                let re = Regex::new(r"^\s*\*\s?").unwrap();
                let cleaned_example = example
                    .lines()
                    .map(|line| re.replace(line, "").to_string())
                    .collect::<Vec<String>>();

                return Some(
                    cleaned_example[..cleaned_example.len() - 1]
                        .join("\n")
                        .trim()
                        .to_string(),
                );
            }
        }
        None
    }
}
