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

pub enum TreeExpandMutation<T> {
  Create { data: T, node: usize },
  Attach { parent_target: usize, node: usize },
}

impl<T: IncrementalBase> From<TreeExpandMutation<T>> for TreeMutation<T> {
  fn from(value: TreeExpandMutation<T>) -> Self {
    match value {
      TreeExpandMutation::Create { data, node } => TreeMutation::Create { data, node },
      TreeExpandMutation::Attach {
        parent_target,
        node,
      } => TreeMutation::Attach {
        parent_target,
        node,
      },
    }
  }
}

impl<T> TreeCollection<T> {
  pub fn expand_with_mapping<U>(
    &self,
    mapper: impl Fn(&T) -> U,
    mut cb: impl FnMut(TreeExpandMutation<U>),
  ) {
    for (handle, node) in &self.nodes.data {
      if node.parent.is_none() {
        let node = self.create_node_ref(handle);
        node.traverse_pair_subtree(|self_node, parent| {
          cb(TreeExpandMutation::Create {
            data: mapper(&self_node.node.data),
            node: self_node.node.handle().index(),
          });
          if let Some(parent) = parent {
            cb(TreeExpandMutation::Attach {
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

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    self.expand_with_mapping(|n| n.clone(), |d| cb(d.into()))
  }
}

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
