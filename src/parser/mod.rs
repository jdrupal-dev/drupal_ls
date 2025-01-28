use tree_sitter::Node;

pub mod yaml;
pub mod php;
pub mod tokens;

pub fn get_closest_parent_by_kind<'a>(node: &'a Node, kind: &'a str) -> Option<Node<'a>> {
    let mut parent = node.parent();
    while parent?.kind() != kind {
        parent = parent?.parent();
    }
    parent
}
