use std::{collections::HashSet, hash::Hash};

pub trait AbstractDirectedGraph {
  fn visit_backward(&self, visitor: impl FnMut(&Self) -> bool);
  fn visit_forward(&self, visitor: impl FnMut(&Self) -> bool);

  /// Visit contains self node, order from the most previous one to self
  ///
  /// Return if contains loop in graph
  fn traverse_dfs_in_topological_order(&self, visitor: &mut impl FnMut(&Self)) -> bool
  where
    Self: Sized + Hash + Eq + Clone,
  {
    struct Ctx<T> {
      unresolved: HashSet<T>,
      visited: HashSet<T>,
    }

    fn visit<T: AbstractDirectedGraph + Hash + Eq + Clone>(
      node: &T,
      ctx: &mut Ctx<T>,
      visitor: &mut impl FnMut(&T),
    ) -> bool {
      if ctx.visited.contains(node) {
        return true;
      }
      if ctx.unresolved.contains(node) {
        return false;
      }

      ctx.unresolved.insert(node.clone());

      node.visit_backward(|from| visit(from, ctx, visitor));

      ctx.unresolved.remove(node);
      ctx.visited.insert(node.clone());
      visitor(node);
      true
    }

    let mut ctx = Ctx {
      unresolved: Default::default(),
      visited: Default::default(),
    };

    visit(self, &mut ctx, visitor)
  }
}
