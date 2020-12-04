use arena_tree::*;

use super::{Element, DIV};

pub type ElementHandle = ArenaTreeNodeHandle<Box<dyn Element>>;

pub struct Document {
  element_tree: ArenaTree<Box<dyn Element>>,
  active_element: Option<ElementHandle>,
  hovering_element: Option<ElementHandle>,
}

impl Document {
  pub fn new() -> Self {
    Self {
      element_tree: ArenaTree::new(Box::new(DIV::new())),
      active_element: None,
      hovering_element: None,
    }
  }
}
