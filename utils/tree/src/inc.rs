use crate::*;
use ::incremental::*;

pub enum TreeMutation<T: IncrementAble> {
  Create(T),
  Delete(TreeNodeHandle<T>),
  Mutate {
    node: TreeNodeHandle<T>,
    delta: T::Delta,
  },
  Attach {
    parent_target: TreeNodeHandle<T>,
    node: TreeNodeHandle<T>,
  },
  Detach {
    node: TreeNodeHandle<T>,
  },
}

pub enum TreeMutationResult<T> {
  Created(TreeNodeHandle<T>),
  Nothing,
}

impl<T: IncrementAble> IncrementAble for TreeCollection<T> {
  type Delta = TreeMutation<T>;
  type DeltaResult = TreeMutationResult<T>;

  fn apply(&mut self, delta: Self::Delta) -> TreeMutationResult<T> {
    match delta {
      TreeMutation::Create(d) => return TreeMutationResult::Created(self.create_node(d)),
      TreeMutation::Delete(d) => self.delete_node(d),
      TreeMutation::Mutate { node, delta } => {
        todo!()
      }
      TreeMutation::Attach {
        parent_target,
        node,
      } => todo!(),
      TreeMutation::Detach { node } => todo!(),
    }
    TreeMutationResult::Nothing
  }
}
