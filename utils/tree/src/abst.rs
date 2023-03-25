use crate::*;

pub struct TreeNodeRef<'a, T> {
  pub tree: &'a TreeCollection<T>,
  pub node: &'a TreeNode<T>,
}

impl<'a, T> Clone for TreeNodeRef<'a, T> {
  fn clone(&self) -> Self {
    Self {
      tree: self.tree,
      node: self.node,
    }
  }
}

impl<'a, T> AbstractTreeNode for TreeNodeRef<'a, T> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    let mut next = self.node.first_child;
    while let Some(next_to_visit) = next {
      let child = self.tree.create_node_ref(next_to_visit);
      visitor(&child);
      next = child.node.next_sibling
    }
  }
}

impl<'a, T> AbstractParentAddressableTreeNode for TreeNodeRef<'a, T> {
  fn get_parent(&self) -> Option<Self> {
    self.node.parent.map(|p| self.tree.create_node_ref(p))
  }
}

impl<T> TreeCollection<T> {
  pub fn create_node_ref(&self, handle: TreeNodeHandle<T>) -> TreeNodeRef<T> {
    TreeNodeRef {
      tree: self,
      node: self.get_node(handle),
    }
  }
}

pub struct TreeNodeMutPtr<T> {
  pub(crate) tree: *mut TreeCollection<T>,
  pub(crate) node: *mut TreeNode<T>,
}

impl<T> Clone for TreeNodeMutPtr<T> {
  fn clone(&self) -> Self {
    Self {
      tree: self.tree,
      node: self.node,
    }
  }
}

impl<T> AbstractTreeMutNode for TreeNodeMutPtr<T> {
  fn visit_children_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
    unsafe {
      let mut next = (*self.node).first_child;
      while let Some(next_to_visit) = next {
        let child = (*self.tree).get_node_mut(next_to_visit) as *mut TreeNode<T>;
        let mut child = TreeNodeMutPtr {
          tree: self.tree,
          node: child,
        };
        visitor(&mut child);
        next = (*child.node).next_sibling
      }
    }
  }
}

impl<T> AbstractParentAddressableMutTreeNode for TreeNodeMutPtr<T> {
  fn get_parent_mut(&mut self) -> Option<Self> {
    unsafe {
      (*self.node).parent.map(|parent| {
        let parent = (*self.tree).get_node_mut(parent) as *mut TreeNode<T>;
        TreeNodeMutPtr {
          tree: self.tree,
          node: parent,
        }
      })
    }
  }
}
