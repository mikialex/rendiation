use crate::*;

pub enum TreeMutation<T: IncrementalBase> {
  Create(T),
  Delete(usize),
  Mutate { node: usize, delta: T::Delta },
  Attach { parent_target: usize, node: usize },
  Detach { node: usize },
}

impl<T: IncrementalBase + Clone> Clone for TreeMutation<T> {
  fn clone(&self) -> Self {
    match self {
      TreeMutation::Create(n) => TreeMutation::Create(n.clone()),
      TreeMutation::Delete(n) => TreeMutation::Delete(*n),
      TreeMutation::Mutate { node, delta } => TreeMutation::Mutate {
        node: *node,
        delta: delta.clone(),
      },
      TreeMutation::Attach {
        parent_target,
        node,
      } => TreeMutation::Attach {
        parent_target: *parent_target,
        node: *node,
      },
      TreeMutation::Detach { node } => TreeMutation::Detach { node: *node },
    }
  }
}

impl<T> IncrementalBase for SharedTreeCollection<T>
where
  T: IncrementalBase,
{
  type Delta = T::Delta;

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    let tree = self.inner.write().unwrap();
    tree.expand(cb);
  }
}
