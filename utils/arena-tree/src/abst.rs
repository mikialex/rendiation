use crate::*;

pub struct ArenaTreeNodeRef<'a, T> {
  pub tree: &'a ArenaTree<T>,
  pub node: &'a ArenaTreeNode<T>,
}

impl<'a, T> Clone for ArenaTreeNodeRef<'a, T> {
  fn clone(&self) -> Self {
    Self {
      tree: self.tree,
      node: self.node,
    }
  }
}

impl<'a, T> AbstractTree for ArenaTreeNodeRef<'a, T> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    // for child in &self.node.children {
    //   visitor(&self.tree.create_node_ref(*child))
    // }
    todo!()
  }
}

impl<'a, T> AbstractParentTree for ArenaTreeNodeRef<'a, T> {
  fn get_parent(&self) -> Option<Self> {
    self.node.parent.map(|p| self.tree.create_node_ref(p))
  }
}

impl<T> ArenaTree<T> {
  pub fn create_node_ref(&self, handle: ArenaTreeNodeHandle<T>) -> ArenaTreeNodeRef<T> {
    ArenaTreeNodeRef {
      tree: self,
      node: self.get_node(handle),
    }
  }
}
