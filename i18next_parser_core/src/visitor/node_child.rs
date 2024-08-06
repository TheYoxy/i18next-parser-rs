use crate::IsEmpty;

/// This enum represents the children of a tag.
pub(super) enum NodeChild {
  /// The children is a text.
  Text(String),
  /// The children is a tag
  Tag(NodeTag),
  /// The children are js code
  Js(String),
}

impl IsEmpty for NodeChild {
  fn is_empty(&self) -> bool {
    match self {
      NodeChild::Text(text) => text.is_empty(),
      NodeChild::Tag(tag) => tag.children.is_none(),
      NodeChild::Js(js) => js.is_empty(),
    }
  }
}

/// This struct represents a tag.
pub(super) struct NodeTag {
  /// The children of the tag.
  pub(super) children: Option<Vec<NodeChild>>,
  /// The name of the tag.
  pub(super) name: String,
  /// A boolean indicating whether the tag is basic.
  pub(super) is_basic: bool,
  /// A boolean indicating whether the tag is self-closing.
  pub(super) self_closing: bool,
}
