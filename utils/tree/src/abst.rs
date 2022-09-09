use crate::*;

pub struct ArenaTreeNodeRef<'a, T> {
  pub tree: &'a TreeCollection<T>,
  pub node: &'a TreeNode<T>,
}

impl<'a, T> Clone for ArenaTreeNodeRef<'a, T> {
  fn clone(&self) -> Self {
    Self {
      tree: self.tree,
      node: self.node,
    }
  }
}

impl<'a, T> AbstractTreeNode for ArenaTreeNodeRef<'a, T> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    let mut next = self.node.first_child;
    while let Some(next_to_visit) = next {
      let child = self.tree.create_node_ref(next_to_visit);
      visitor(&child);
      next = child.node.next_sibling
    }
  }
}

impl<'a, T> AbstractParentAddressableTreeNode for ArenaTreeNodeRef<'a, T> {
  fn get_parent(&self) -> Option<Self> {
    self.node.parent.map(|p| self.tree.create_node_ref(p))
  }
}

impl<T> TreeCollection<T> {
  pub fn create_node_ref(&self, handle: TreeNodeHandle<T>) -> ArenaTreeNodeRef<T> {
    ArenaTreeNodeRef {
      tree: self,
      node: self.get_node(handle),
    }
  }
}

pub struct ArenaTreeNodeMutPtr<T> {
  pub(crate) tree: *mut TreeCollection<T>,
  pub(crate) node: *mut TreeNode<T>,
}

/// todo test it with miri
impl<T> AbstractTreePairMutNode for ArenaTreeNodeMutPtr<T> {
  /// parent child
  fn visit_self_child_pair_mut(&mut self, mut visitor: impl FnMut(&mut Self, &mut Self)) {
    unsafe {
      let mut next = (*self.node).first_child;
      while let Some(next_to_visit) = next {
        let child = (*self.tree).get_node_mut(next_to_visit) as *mut TreeNode<T>;
        let mut child = ArenaTreeNodeMutPtr {
          tree: self.tree,
          node: child,
        };
        visitor(self, &mut child);
        next = (*child.node).next_sibling
      }
    }
  }
}
