use std::ops::Deref;

use incremental::ApplicableIncremental;

use crate::*;

#[derive(Clone)]
pub enum TreeMutation<T: IncrementalBase> {
  Create { data: T, node: usize },
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
          cb(TreeMutation::Create {
            data: mapper(&self_node.node.data),
            node: self_node.node.handle().index(),
          });
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

// impl<T, X> IncrementalBase for ReactiveTreeCollection<T, X>
// where
//   T: Send + Sync + 'static,
//   X: IncrementalBase + Clone,
//   T: std::ops::Deref<Target = X>,
// {
//   type Delta = TreeMutation<X>;

//   fn expand(&self, cb: impl FnMut(Self::Delta)) {
//     self.inner.expand_with_mapping(|n| n.deref().clone(), cb)
//   }
// }

#[derive(Debug)]
pub enum TreeDeltaMutationError<T> {
  Inner(T),
  Mutation(TreeMutationError),
  InputHandleNotMatchInsertResult,
}

impl<T: ApplicableIncremental + Clone> ApplicableIncremental for TreeCollection<T> {
  type Error = TreeDeltaMutationError<T::Error>;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      TreeMutation::Create { data, node } => {
        let handle = self.create_node(data);
        (handle.index() == node)
          .then_some(())
          .ok_or(TreeDeltaMutationError::InputHandleNotMatchInsertResult)
      }
      TreeMutation::Delete(idx) => {
        let handle = self.recreate_handle(idx);
        self.delete_node(handle); // todo
        Ok(())
      }
      TreeMutation::Mutate { node, delta } => {
        let handle = self.recreate_handle(node);
        let node = self.get_node_data_mut(handle);
        node.apply(delta).map_err(TreeDeltaMutationError::Inner)
      }
      TreeMutation::Attach {
        parent_target,
        node,
      } => self
        .node_add_child_by(
          self.recreate_handle(parent_target),
          self.recreate_handle(node),
        )
        .map_err(TreeDeltaMutationError::Mutation),
      TreeMutation::Detach { node } => self
        .node_detach_parent(self.recreate_handle(node))
        .map_err(TreeDeltaMutationError::Mutation),
    }
  }
}

impl<T> IncrementalBase for SharedTreeCollection<T>
where
  T: IncrementalBase,
{
  type Delta = T::Delta;

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    self.inner.deref().expand(cb);
  }
}
