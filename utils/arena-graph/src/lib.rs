use arena::*;
use std::collections::BTreeSet;

pub struct ArenaGraph<T> {
  nodes_data: Arena<T>,
  nodes: Arena<ArenaGraphNode<T>>,
}

pub type ArenaGraphNodeHandle<T> = Handle<ArenaGraphNode<T>>;

pub struct ArenaGraphNode<T> {
  data_handle: Handle<T>,
  handle: ArenaGraphNodeHandle<T>,
  from: BTreeSet<ArenaGraphNodeHandle<T>>,
  to: BTreeSet<ArenaGraphNodeHandle<T>>,
}

impl<T> ArenaGraphNode<T> {
  pub fn handle(&self) -> ArenaGraphNodeHandle<T> {
    self.handle
  }

  pub fn data_handle(&self) -> Handle<T> {
    self.data_handle
  }

  pub fn new(data_handle: Handle<T>, handle: ArenaGraphNodeHandle<T>) -> Self {
    Self {
      data_handle,
      handle,
      from: BTreeSet::new(),
      to: BTreeSet::new(),
    }
  }
}

impl<T> ArenaGraph<T> {
  pub fn new() -> Self {
    Self {
      nodes_data: Arena::new(),
      nodes: Arena::new(),
    }
  }

  pub fn new_node(&mut self, node_data: T) -> ArenaGraphNodeHandle<T> {
    let handle = self.nodes_data.insert(node_data);
    let node = ArenaGraphNode::new(handle, Handle::from_raw_parts(0, 0));
    let handle = self.nodes.insert(node);
    self.nodes.get_mut(handle).unwrap().handle = handle; // improvements need
    handle
  }

  pub fn connect_node(&mut self, from: Handle<ArenaGraphNode<T>>, to: Handle<ArenaGraphNode<T>>) {
    let from_node = self.nodes.get_mut(from).unwrap();
    from_node.to.insert(to);

    let to_node = self.nodes.get_mut(to).unwrap();
    to_node.from.insert(from);
  }
}
