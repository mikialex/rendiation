use arena::*;

pub struct ArenaTree<T> {
  nodes: Arena<ArenaTreeNode<T>>,
}
pub type ArenaTreeNodeHandle<T> = Handle<ArenaTreeNode<T>>;

pub struct ArenaTreeNode<T> {
  pub data: T,
  handle: ArenaTreeNodeHandle<T>,
  parent: Option<ArenaTreeNodeHandle<T>>,
  children: Vec<ArenaTreeNodeHandle<T>>,
}

impl<T> ArenaTreeNode<T> {
  fn new(data: T) -> Self {
    Self {
      data,
      handle: Handle::from_raw_parts(0, 0),
      parent: None,
      children: Vec::new(),
    }
  }

  pub fn add(&mut self, child_to_add: &mut ArenaTreeNode<T>) -> &mut Self {
    if child_to_add.parent.is_some() {
      panic!("child node already has a parent");
    }
    child_to_add.parent = Some(self.handle);
    self.children.push(child_to_add.handle);
    self
  }

  pub fn remove(&mut self, child_to_remove: &mut ArenaTreeNode<T>) -> &mut Self {
    let child_index = self
      .children
      .iter()
      .position(|&x| x == child_to_remove.handle)
      .expect("tried to remove nonexistent child");

    self.children.swap_remove(child_index);
    self
  }
}

impl<T> ArenaTree<T> {
  pub fn new(root_data: T) -> Self {
    let mut tree = Self {
      nodes: Arena::new(),
    };
    tree.create_node(root_data);
    tree
  }

  pub fn create_node(&mut self, node_data: T) -> ArenaTreeNodeHandle<T> {
    let node = ArenaTreeNode::new(node_data);
    let handle = self.nodes.insert(node);
    self.nodes.get_mut(handle).unwrap().handle = handle;
    handle
  }

  pub fn get_node(&self, handle: ArenaTreeNodeHandle<T>) -> &ArenaTreeNode<T> {
    self.nodes.get(handle).unwrap()
  }

  pub fn get_node_mut(&mut self, handle: ArenaTreeNodeHandle<T>) -> &mut ArenaTreeNode<T> {
    self.nodes.get_mut(handle).unwrap()
  }

  pub fn node_add_child_by_id(
    &mut self,
    parent_id: ArenaTreeNodeHandle<T>,
    child_id: ArenaTreeNodeHandle<T>,
  ) {
    let (parent, child) = self.get_parent_child_pair(parent_id, child_id);
    parent.add(child);
  }

  pub fn node_remove_child_by_id(
    &mut self,
    parent_id: ArenaTreeNodeHandle<T>,
    child_id: ArenaTreeNodeHandle<T>,
  ) {
    let (parent, child) = self.get_parent_child_pair(parent_id, child_id);
    parent.remove(child);
  }

  // pub fn add_to_root(&mut self, child_id: ArenaTreeNodeHandle<T>) {
  //   todo!()
  //   // self.node_add_child_by_id(self.root, child_id);
  // }

  pub fn get_parent_child_pair(
    &mut self,
    parent_id: ArenaTreeNodeHandle<T>,
    child_id: ArenaTreeNodeHandle<T>,
  ) -> (&mut ArenaTreeNode<T>, &mut ArenaTreeNode<T>) {
    let (parent, child) = self.nodes.get2_mut(parent_id, child_id);
    (parent.unwrap(), child.unwrap())
  }

  pub fn traverse(
    &mut self,
    start_index: ArenaTreeNodeHandle<T>,
    visit_stack: &mut Vec<ArenaTreeNodeHandle<T>>,
    mut visitor: impl FnMut(&mut ArenaTreeNode<T>, Option<&mut ArenaTreeNode<T>>),
  ) {
    visit_stack.clear();
    visit_stack.push(start_index);

    while let Some(index) = visit_stack.pop() {
      if let Some(parent_index) = self.get_node(index).parent {
        let (parent, this) = self.get_parent_child_pair(parent_index, index);
        visitor(this, Some(parent));
        visit_stack.extend(this.children.iter().cloned())
      } else {
        let this = self.get_node_mut(index);
        visitor(this, None);
        visit_stack.extend(this.children.iter().cloned())
      }
    }
  }
}
