use std::collections::BTreeSet;

pub use arena::*;

pub struct ArenaGraph<T> {
  nodes: Arena<ArenaGraphNode<T>>,
}

pub type ArenaGraphNodeHandle<T> = Handle<ArenaGraphNode<T>>;

pub struct ArenaGraphNode<T> {
  data: T,
  handle: ArenaGraphNodeHandle<T>,
  from: Vec<ArenaGraphNodeHandle<T>>,
  to: Vec<ArenaGraphNodeHandle<T>>,
}

impl<T> ArenaGraphNode<T> {
  pub fn data(&self) -> &T {
    &self.data
  }

  pub fn data_mut(&mut self) -> &mut T {
    &mut self.data
  }

  pub fn handle(&self) -> ArenaGraphNodeHandle<T> {
    self.handle
  }

  pub fn from(&self) -> &Vec<ArenaGraphNodeHandle<T>> {
    &self.from
  }

  pub fn to(&self) -> &Vec<ArenaGraphNodeHandle<T>> {
    &self.to
  }

  fn new(data: T) -> Self {
    Self {
      data,
      handle: Handle::from_raw_parts(0, 0),
      from: Vec::new(),
      to: Vec::new(),
    }
  }
}

impl<T> ArenaGraph<T> {
  pub fn new() -> Self {
    Self {
      nodes: Arena::new(),
    }
  }

  pub fn create_node(&mut self, node_data: T) -> ArenaGraphNodeHandle<T> {
    let node = ArenaGraphNode::new(node_data);
    let handle = self.nodes.insert(node);
    self.nodes.get_mut(handle).unwrap().handle = handle;
    handle
  }

  pub fn get_node(&self, handle: ArenaGraphNodeHandle<T>) -> &ArenaGraphNode<T> {
    self.nodes.get(handle).unwrap()
  }

  pub fn get_node_mut(&mut self, handle: ArenaGraphNodeHandle<T>) -> &mut ArenaGraphNode<T> {
    self.nodes.get_mut(handle).unwrap()
  }

  pub fn connect_node(&mut self, from: ArenaGraphNodeHandle<T>, to: ArenaGraphNodeHandle<T>) {
    let from_node = self.nodes.get_mut(from).unwrap();
    from_node.to.push(to);

    let to_node = self.nodes.get_mut(to).unwrap();
    to_node.from.push(from);
  }

  /// visit contains self node
  pub fn traverse_dfs_in_topological_order(
    &self,
    node: ArenaGraphNodeHandle<T>,
    visitor: &mut impl FnMut(&ArenaGraphNode<T>),
    if_loop_exist: &mut impl FnMut(),
  ) {
    let mut unresolved: BTreeSet<ArenaGraphNodeHandle<T>> = BTreeSet::new();
    let mut visited: BTreeSet<ArenaGraphNodeHandle<T>> = BTreeSet::new();

    fn visit<T>(
      n_handle: ArenaGraphNodeHandle<T>,
      visited: &mut BTreeSet<ArenaGraphNodeHandle<T>>,
      unresolved: &mut BTreeSet<ArenaGraphNodeHandle<T>>,
      graph: &ArenaGraph<T>,
      visitor: &mut impl FnMut(&ArenaGraphNode<T>),
      if_loop_exist: &mut impl FnMut(),
    ) -> bool {
      if visited.contains(&n_handle) {
        return true;
      }
      if unresolved.contains(&n_handle) {
        if_loop_exist();
        return false;
      }

      unresolved.insert(n_handle);

      let node = graph.get_node(n_handle);
      node
        .from
        .iter()
        .take_while(|&&from_id| visit(from_id, visited, unresolved, graph, visitor, if_loop_exist))
        .for_each(|_| {});

      unresolved.remove(&n_handle);
      visited.insert(n_handle);
      visitor(node);
      true
    }

    visit(
      node,
      &mut visited,
      &mut unresolved,
      self,
      visitor,
      if_loop_exist,
    );
  }

  pub fn topological_order_list(
    &self,
    node: ArenaGraphNodeHandle<T>,
  ) -> Option<Vec<ArenaGraphNodeHandle<T>>> {
    let mut list = Vec::new();
    let mut looped = false;
    self.traverse_dfs_in_topological_order(node, &mut |node| list.push(node.handle()), &mut || {
      looped = true
    });
    if looped {
      None
    } else {
      Some(list)
    }
  }
}
