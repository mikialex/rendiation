use std::{collections::HashSet, hash::Hash};

pub trait AbstractDirectedGraph {
  fn visit_backward(&self, visitor: impl FnMut(&Self));
  fn visit_forward(&self, visitor: impl FnMut(&Self));

  /// Visit contains self node, order from the most previous one to self
  fn traverse_dfs_in_topological_order(
    &self,
    visitor: &mut impl FnMut(&Self),
    if_loop_exist: &mut impl FnMut(),
  ) where
    Self: Sized + Hash + Eq + Clone,
  {
    let mut unresolved = HashSet::new();
    let mut visited = HashSet::new();

    fn visit<T: AbstractDirectedGraph + Hash + Eq + Clone>(
      node: &T,
      visited: &mut HashSet<T>,
      unresolved: &mut HashSet<T>,
      visitor: &mut impl FnMut(&T),
      if_loop_exist: &mut impl FnMut(),
    ) -> bool {
      if visited.contains(node) {
        return true;
      }
      if unresolved.contains(node) {
        if_loop_exist();
        return false;
      }

      unresolved.insert(node.clone());

      let mut found_loop = false;
      node.visit_backward(|from| {
        if !found_loop {
          found_loop = visit(from, visited, unresolved, visitor, if_loop_exist);
        }
      });

      unresolved.remove(node);
      visited.insert(node.clone());
      visitor(node);
      true
    }

    visit(self, &mut visited, &mut unresolved, visitor, if_loop_exist);
  }
}
