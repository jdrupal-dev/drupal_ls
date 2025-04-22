use lsp_types::Position;
use std::collections::HashMap;
use std::vec;
use tree_sitter::{Node, Point};

use super::{get_node_at_position, get_tree, position_to_point, tokens::{
    DrupalPermission, DrupalRoute, DrupalRouteDefaults, DrupalService, PhpClassName, PhpMethod,
    Token, TokenData,
}};

pub struct YamlParser {
    source: String,
    uri: String,
}

impl YamlParser {
    pub fn new(source: &str, uri: &str) -> Self {
        Self {
            source: source.to_string(),
            uri: uri.to_string(),
        }
    }

    pub fn get_tokens(&self) -> Vec<Token> {
        let tree = get_tree(&self.source, &tree_sitter_yaml::language());
        self.parse_nodes(vec![tree.unwrap().root_node()])
    }

    pub fn get_token_at_position(&self, position: Position) -> Option<Token> {
        let tree = get_tree(&self.source, &tree_sitter_yaml::language())?;
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

        let mut current_nodes: Box<Vec<Node>> = Box::new(nodes.clone());
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
            "block_mapping_pair" => self.parse_block_mapping_pair(node, point),
            _ => None,
        }
    }

    fn parse_block_mapping_pair(&self, node: Node, point: Option<Point>) -> Option<Token> {
        let key_node = node.child_by_field_name("key")?;
        let key = self.get_node_text(&key_node);
        let value_node = node.child_by_field_name("value")?;

        if let Some(map) = self.get_block_node_map(&value_node) {
            // Parse Drupal Permission.
            if self.uri.ends_with(".permissions.yml") {
                if let Some(title) = map.get("title") {
                    return Some(Token::new(
                        TokenData::DrupalPermissionDefinition(DrupalPermission {
                            name: key.to_string(),
                            title: self.get_node_text(title).to_string(),
                        }),
                        node.range(),
                    ));
                }
            }
            // Parse Drupal Route.
            else if let (Some(path), Some(defaults)) = (map.get("path"), map.get("defaults")) {
                return Some(Token::new(
                    TokenData::DrupalRouteDefinition(DrupalRoute {
                        name: key.to_string(),
                        path: self.get_node_text(path).to_string(),
                        _defaults: self.parse_route_defaults(defaults)?,
                    }),
                    node.range(),
                ));
            }
            // Parse Drupal Service.
            else if let Some(class) = map.get("class") {
                return Some(Token::new(
                    TokenData::DrupalServiceDefinition(DrupalService {
                        name: key.to_string(),
                        class: PhpClassName::from(self.get_node_text(class)),
                    }),
                    node.range(),
                ));
            }
        }

        match key {
            "_controller" => Some(Token::new(
                TokenData::PhpMethodReference(
                    PhpMethod::try_from(self.get_node_text(&value_node)).ok()?,
                ),
                value_node.range(),
            )),
            "_form" | "class" => Some(Token::new(
                TokenData::PhpClassReference(PhpClassName::from(self.get_node_text(&value_node))),
                value_node.range(),
            )),
            "_permission" => Some(Token::new(
                TokenData::DrupalPermissionReference(
                    self.get_node_text(&value_node).to_string().replace("'", ""),
                ),
                value_node.range(),
            )),
            "route_name" => Some(Token::new(
                TokenData::DrupalRouteReference(
                    self.get_node_text(&value_node).to_string().replace("'", ""),
                ),
                value_node.range(),
            )),
            "arguments" => {
                let argument = value_node.descendant_for_point_range(point?, point?)?;
                if argument.kind() != "single_quote_scalar" {
                    return None;
                }

                let argument_string = self
                    .get_node_text(&argument)
                    .to_string()
                    .trim_matches(['\'', '@'])
                    .to_string();

                Some(Token::new(
                    TokenData::DrupalServiceReference(argument_string),
                    value_node.range(),
                ))
            }
            _ => None,
        }
    }

    fn parse_route_defaults(&self, node: &Node) -> Option<DrupalRouteDefaults> {
        let map = self.get_block_node_map(node)?;
        Some(DrupalRouteDefaults {
            _controller: map
                .get("_controller")
                .and_then(|node| PhpMethod::try_from(self.get_node_text(node)).ok()),
            _form: map
                .get("_form")
                .map(|node| PhpClassName::from(self.get_node_text(node))),
            _entity_form: map
                .get("_form")
                .map(|node| self.get_node_text(node).to_string()),
            _title: map
                .get("_title")
                .map(|node| self.get_node_text(node).to_string()),
        })
    }

    fn get_block_node_map<'a>(&'a self, node: &'a Node) -> Option<HashMap<&'a str, Node<'a>>> {
        if node.kind() != "block_node" {
            return None;
        }

        let mut result: HashMap<&str, Node<'a>> = HashMap::new();
        let mut cursor = node.walk();
        node.child(0)?.children(&mut cursor).for_each(|child| {
            if let (Some(key), Some(value)) = (
                child.child_by_field_name("key"),
                child.child_by_field_name("value"),
            ) {
                result.insert(self.get_node_text(&key), value);
            }
        });
        Some(result)
    }

    fn get_node_text(&self, node: &Node) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }
}
