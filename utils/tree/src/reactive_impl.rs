use ::reactive::EventSource;

use crate::*;

pub struct ReactiveTreeCollection<T: IncrementalBase> {
  inner: TreeCollection<T>,
  source: EventSource<TreeMutation<T>>,
}

impl<T: IncrementalBase + Clone> CoreTree for ReactiveTreeCollection<T> {
  type Node = T;
  type Handle = TreeNodeHandle<T>;

  fn get_node_data(&self, handle: Self::Handle) -> &Self::Node {
    self.inner.get_node_data(handle)
  }

  fn get_node_data_mut(&mut self, handle: Self::Handle) -> &mut Self::Node {
    // mutation is emit by hand
    self.inner.get_node_data_mut(handle)
  }

  fn create_node(&mut self, data: Self::Node) -> Self::Handle {
    self.source.emit(&TreeMutation::Create(data.clone()));
    self.inner.create_node(data)
  }

  fn delete_node(&mut self, handle: Self::Handle) {
    self.source.emit(&TreeMutation::Delete(handle.index()));
    self.inner.delete_node(handle)
  }

  fn node_add_child_by(
    &mut self,
    parent: Self::Handle,
    child_to_attach: Self::Handle,
  ) -> Result<(), TreeMutationError> {
    self.source.emit(&TreeMutation::Attach {
      parent_target: parent.index(),
      node: child_to_attach.index(),
    });
    self.inner.node_add_child_by(parent, child_to_attach)
  }

  fn node_detach_parent(&mut self, child_to_detach: Self::Handle) -> Result<(), TreeMutationError> {
    self.source.emit(&TreeMutation::Detach {
      node: child_to_detach.index(),
    });
    self.inner.node_detach_parent(child_to_detach)
  }
}
