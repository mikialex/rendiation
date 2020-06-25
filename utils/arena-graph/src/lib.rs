use arena::*;

pub struct ArenaGraph<T> {
  nodes_data: Arena<T>,
  nodes: Arena<ArenaGraphNode<T>>,
}

pub struct ArenaGraphNode<T>{
  handle: Handle<T>,
  from: Vec<Handle<T>>
}

impl<T> ArenaGraphNode<T> {
  pub fn new(handle: Handle<T>) -> Self {
    Self {
      handle, 
      from: Vec::new()
    }
  }
}

impl<T> ArenaGraph<T> {
  pub fn new() -> Self {
    Self {
      nodes_data: Arena::new(),
      nodes: Arena::new()
    }
  }

  pub fn new_node(&mut self, node_data: T) -> Handle<ArenaGraphNode<T>> {
    let handle = self.nodes_data.insert(node_data);
    let node = ArenaGraphNode::new(handle);
    self.nodes.insert(node)
  }
}