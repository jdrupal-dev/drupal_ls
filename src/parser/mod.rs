pub mod php;
pub mod tokens;
pub mod yaml;

use lsp_types::Position;
use tree_sitter::{Language, Node, Parser, Point, Tree};

pub fn get_closest_parent_by_kind<'a>(node: &'a Node, kind: &'a str) -> Option<Node<'a>> {
    let mut parent = node.parent();
    while parent?.kind() != kind {
        parent = parent?.parent();
    }
    parent
}

pub fn get_tree(source: &str, language: &Language) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(language).ok()?;
    parser.parse(source.as_bytes(), None)
}

pub fn get_node_at_position(tree: &Tree, position: Position) -> Option<Node> {
    let start = position_to_point(position);
    tree.root_node().descendant_for_point_range(start, start)
}

pub fn position_to_point(position: Position) -> Point {
    Point::new(position.line as usize, position.character as usize)
}
