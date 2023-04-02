use crate::*;

#[derive(Clone)]
pub enum TreeMutation<T: IncrementalBase> {
  Create(T),
  Delete(usize),
  Mutate { node: usize, delta: T::Delta },
  Attach { parent_target: usize, node: usize },
  Detach { node: usize },
}

impl<T> TreeCollection<T> {
  pub fn expand_with_mapping<U: IncrementalBase>(
    &self,
    mapper: impl Fn(&T) -> U,
    mut cb: impl FnMut(TreeMutation<U>),
  ) {
    for (handle, node) in &self.nodes.data {
      if node.parent.is_none() {
        let node = self.create_node_ref(handle);
        node.traverse_pair_subtree(|self_node, parent| {
          cb(TreeMutation::Create(mapper(&self_node.node.data)));
          if let Some(parent) = parent {
            cb(TreeMutation::Attach {
              parent_target: parent.node.handle().index(),
              node: self_node.node.handle().index(),
            });
          }
          NextTraverseVisit::VisitChildren
        })
      }
    }
  }
}
impl<T: IncrementalBase + Clone> IncrementalBase for TreeCollection<T> {
  type Delta = TreeMutation<T>;

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    self.expand_with_mapping(|n| n.clone(), cb)
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
