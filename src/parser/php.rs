use std::collections::HashMap;

use lsp_types::Position;
use tree_sitter::{Node, Parser, Point, Tree};

use super::get_closest_parent_by_kind;
use super::tokens::{DrupalHook, PhpClass, PhpClassName, PhpMethod, Token, TokenData};

pub struct PhpParser {
    source: String,
}

// TODO: A lot of code has been copied from the yaml parser.
// How can we DRY this up?
impl PhpParser {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
        }
    }

    pub fn get_tree(&self) -> Option<Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .ok()?;
        parser.parse(self.source.as_bytes(), None)
    }

    pub fn get_tokens(&self) -> Vec<Token> {
        let tree = self.get_tree();
        self.parse_nodes(vec![tree.unwrap().root_node()])
    }

    pub fn get_token_at_position(&self, position: Position) -> Option<Token> {
        let tree = self.get_tree()?;
        let mut node = self.get_node_at_position(&tree, position)?;
        let point = self.position_to_point(position);

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

    fn get_node_at_position<'a>(&self, tree: &'a Tree, position: Position) -> Option<Node<'a>> {
        let start = self.position_to_point(position);
        tree.root_node().descendant_for_point_range(start, start)
    }

    fn position_to_point(&self, position: Position) -> Point {
        Point::new(position.line as usize, position.character as usize)
    }

    fn parse_nodes(&self, nodes: Vec<Node>) -> Vec<Token> {
        let mut tokens: Vec<Token> = vec![];

        let mut current_nodes: Box<Vec<Node>> = Box::new(nodes);
        while current_nodes.len() > 0 {
            let mut new_nodes: Box<Vec<Node>> = Box::new(Vec::new());
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
            "scoped_call_expression" | "member_call_expression" => {
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
        if text.find("Implements hook_").is_some() {
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
        if string_content.kind() != "string_content" {
            return None;
        }

        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(&name_node);

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
        if name == "get" {
            let object_node = node.child_by_field_name("object")?;
            if self.get_node_text(&object_node) == "$container" {
                return Some(Token::new(
                    TokenData::DrupalServiceReference(
                        self.get_node_text(&string_content).to_string(),
                    ),
                    node.range(),
                ));
            }
        }

        None
    }

    fn parse_class_declaration(&self, node: Node) -> Option<Token> {
        let mut methods: HashMap<String, Box<Token>> = HashMap::new();
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            body_node.children(&mut cursor).for_each(|child| {
                if let Some(token) = self.parse_method_declaration(child) {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        methods.insert(self.get_node_text(&name_node).to_string(), Box::new(token));
                    }
                }
            });
        }

        let token = Token::new(
            TokenData::PhpClassDefinition(PhpClass {
                name: self.get_class_name_from_node(node)?,
                methods,
            }),
            node.range(),
        );
        Some(token)
    }

    fn parse_method_declaration(&self, node: Node) -> Option<Token> {
        if node.kind() != "method_declaration" {
            return None;
        }

        let class_node = get_closest_parent_by_kind(&node, "class_declaration")?;

        let name_node = node.child_by_field_name("name")?;
        return Some(Token::new(
            TokenData::PhpMethodDefinition(PhpMethod {
                name: self.get_node_text(&name_node).to_string(),
                class_name: self.get_class_name_from_node(class_node)?,
            }),
            node.range(),
        ));
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
}
