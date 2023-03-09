use crate::*;
use ::incremental::*;

pub enum SharedTreeMutation<T: IncrementalBase> {
  Create(NodeRef<T>),
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

impl<T: IncrementalBase> Clone for SharedTreeMutation<T> {
  fn clone(&self) -> Self {
    match self {
      SharedTreeMutation::Create(n) => SharedTreeMutation::Create(n.clone()),
      SharedTreeMutation::Delete(n) => SharedTreeMutation::Delete(*n),
      SharedTreeMutation::Mutate { node, delta } => SharedTreeMutation::Mutate {
        node: *node,
        delta: delta.clone(),
      },
      SharedTreeMutation::Attach {
        parent_target,
        node,
      } => SharedTreeMutation::Attach {
        parent_target: *parent_target,
        node: *node,
      },
      SharedTreeMutation::Detach { node } => SharedTreeMutation::Detach { node: *node },
    }
  }
}

impl<T: IncrementalBase + Send + Sync> IncrementalBase for SharedTreeCollection<T> {
  type Delta = SharedTreeMutation<T>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    let tree = self.inner.write().unwrap();
    for (handle, node) in &tree.nodes.data {
      if node.first_child.is_none() {
        let node = tree.create_node_ref(handle);
        // todo fix traverse_pair skip leaf/parent node
        node.traverse_pair(&mut |self_node, parent| {
          cb(SharedTreeMutation::Create(NodeRef {
            nodes: self.clone(),
            handle: self_node.node.handle(),
          }));
          cb(SharedTreeMutation::Attach {
            parent_target: parent.node.handle(),
            node: self_node.node.handle(),
          });
        })
      }
    }
  }
}
