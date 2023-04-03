mod iter;
pub use iter::*;

pub enum NextTraverseVisit {
  /// exit tree traverse
  Exit,
  /// continue children
  VisitChildren,
  /// continue siblings, skip children
  SkipChildren,
}

pub trait AbstractTreeNode {
  fn visit_children(&self, visitor: impl FnMut(&Self));
  fn children_count(&self) -> usize {
    let mut visit_count = 0;
    self.visit_children(|_| visit_count += 1);
    visit_count
  }
  fn has_children(&self) -> bool {
    self.children_count() == 0
  }
  /// Check if this node is a leaf of the tree.
  fn is_leaf(&self) -> bool {
    !self.has_children()
  }

  fn traverse(&self, visitor: &mut impl FnMut(&Self)) {
    visitor(self);
    self.visit_children(|child| child.traverse(visitor))
  }

  /// leaf_visitor: when meet leaf, return if should continue visit tree
  fn traverse_by_branch_leaf(
    &self,
    mut branch_enter_visitor: impl FnMut(&Self) -> NextTraverseVisit,
    mut leaf_visitor: impl FnMut(&Self) -> bool,
  ) where
    Self: Clone,
  {
    let mut stack = Vec::new(); // todo estimate depth for allocation
    stack.push(self.clone());

    while let Some(node) = stack.pop() {
      match branch_enter_visitor(&node) {
        NextTraverseVisit::Exit => return,
        NextTraverseVisit::VisitChildren => {
          if node.is_leaf() {
            if !leaf_visitor(&node) {
              return;
            }
          } else {
            node.visit_children(|child| stack.push(child.clone()));
          }
        }
        NextTraverseVisit::SkipChildren => {}
      }
    }
  }

  fn traverse_pair_subtree(
    &self,
    mut visitor: impl FnMut(&Self, Option<&Self>) -> NextTraverseVisit,
  ) where
    Self: AbstractParentAddressableTreeNode + Clone,
  {
    use NextTraverseVisit::*;
    let mut stack = Vec::new();
    stack.push(self.clone());

    while let Some(node) = stack.pop() {
      let next = if let Some(parent) = node.get_parent() {
        visitor(&node, Some(&parent))
      } else {
        visitor(&node, None)
      };

      match next {
        Exit => return,
        VisitChildren => node.visit_children(|child| stack.push(child.clone())),
        SkipChildren => continue,
      };
    }
  }

  /// Get the tree depth starting with this node. (not including self)
  fn get_max_children_depth(&self) -> usize {
    if self.is_leaf() {
      return 0;
    }

    let mut max_depth = 0;
    self.visit_children(|child| max_depth = max_depth.max(child.get_max_children_depth()));
    max_depth + 1
  }

  /// for every node in subtree, the visit_decider
  fn traverse_iter<F>(&self, visit_decider: F) -> TraverseIter<Self, F>
  where
    Self: Sized + Clone,
    F: FnMut(&Self) -> NextTraverseVisit,
  {
    TraverseIter {
      visit_stack: vec![self.clone()],
      visit_decider,
    }
  }
}

pub trait AbstractTreeMutNode {
  fn visit_children_mut(&mut self, visitor: impl FnMut(&mut Self));

  fn traverse_mut(&mut self, visitor: &mut impl FnMut(&mut Self)) {
    visitor(self);
    self.visit_children_mut(|child| child.traverse_mut(visitor))
  }

  fn traverse_pair_subtree_mut(
    &mut self,
    visitor: &mut impl FnMut(&mut Self, Option<&mut Self>) -> NextTraverseVisit,
  ) where
    Self: AbstractParentAddressableMutTreeNode + Clone,
  {
    use NextTraverseVisit::*;
    let mut stack = Vec::new();
    stack.push(self.clone());

    while let Some(mut node) = stack.pop() {
      let next = if let Some(mut parent) = node.get_parent_mut() {
        visitor(&mut node, Some(&mut parent))
      } else {
        visitor(&mut node, None)
      };

      match next {
        Exit => return,
        VisitChildren => node.visit_children_mut(|child| stack.push(child.clone())),
        SkipChildren => continue,
      };
    }
  }

  /// for every node in subtree, the visit_decider
  fn traverse_iter_mut<F>(&self, visit_decider: F) -> TraverseMutIter<Self, F>
  where
    Self: Sized + Clone,
    F: FnMut(&Self) -> NextTraverseVisit,
  {
    TraverseMutIter {
      visit_stack: vec![self.clone()],
      visit_decider,
    }
  }
}

pub trait AbstractParentAddressableTreeNode: Sized {
  /// this actually requires self is cheap to create/clone
  fn get_parent(&self) -> Option<Self>;

  fn get_tree_depth(&self) -> usize {
    let mut count = 0;
    self.traverse_parent(|_| {
      count += 1;
      true
    });
    count - 1
  }

  /// contains self node
  ///
  /// return if should visit parent
  fn traverse_parent(&self, mut visitor: impl FnMut(&Self) -> bool) {
    let should_visit_parent = visitor(self);
    if should_visit_parent {
      if let Some(parent) = self.get_parent() {
        parent.traverse_parent(visitor)
      }
    }
  }

  /// contains self node
  ///
  /// return if should visit parent
  fn traverse_pair_parent_chain(&self, mut visitor: impl FnMut(&Self, Option<&Self>) -> bool) {
    if let Some(parent) = self.get_parent() {
      if visitor(self, Some(&parent)) {
        parent.traverse_pair_parent_chain(visitor)
      }
    } else {
      visitor(self, None);
    }
  }
}

pub trait AbstractParentAddressableMutTreeNode: Sized {
  /// this actually requires self is cheap to create/clone
  fn get_parent_mut(&mut self) -> Option<Self>;

  /// contains self node
  ///
  /// return if should visit parent
  fn traverse_parent_mut(&mut self, mut visitor: impl FnMut(&mut Self) -> bool) {
    let should_visit_parent = visitor(self);
    if should_visit_parent {
      if let Some(mut parent) = self.get_parent_mut() {
        parent.traverse_parent_mut(visitor)
      }
    }
  }

  /// contains self node
  ///
  /// return if should visit parent
  fn traverse_pair_parent_chain_mut(
    &mut self,
    visitor: &mut impl FnMut(&Self, Option<&Self>) -> bool,
  ) {
    if let Some(mut parent) = self.get_parent_mut() {
      if visitor(self, Some(&parent)) {
        parent.traverse_pair_parent_chain_mut(visitor)
      }
    } else {
      visitor(self, None);
    }
  }
}
