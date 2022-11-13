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

impl<T: IncrementAble> IncrementAble for TreeCollection<T> {
  type Delta = TreeMutation<T>;
  type Error = ();

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      TreeMutation::Create(d) => {
        // question, how do we handle return the handle??
        self.create_node(d);
      }
      TreeMutation::Delete(d) => self.delete_node(d),
      TreeMutation::Mutate { node, delta } => {
        let node = self.get_node_mut(node).data_mut();
        node.apply(delta).unwrap();
      }
      TreeMutation::Attach {
        parent_target,
        node,
      } => self.node_add_child_by(parent_target, node).unwrap(),
      TreeMutation::Detach { node } => {
        self.node_detach_parent(node).unwrap();
      }
    }
    Ok(())
  }

  fn expand(&self, _cb: impl FnMut(Self::Delta)) {
    todo!()
  }
}
