use arena::*;

pub struct ArenaTree<T> {
  nodes_data: Arena<T>,
  nodes: Arena<ArenaTreeNode<T>>,
}
pub type ArenaTreeNodeHandle<T> = Handle<ArenaTreeNode<T>>;

pub struct ArenaTreeNode<T> {
  data_handle: Handle<T>,
  handle: ArenaTreeNodeHandle<T>,
  parent: Option<ArenaTreeNodeHandle<T>>,
  children: Vec<ArenaTreeNodeHandle<T>>,
}

impl<T> ArenaTreeNode<T> {
  pub fn new(data_handle: Handle<T>) -> Self {
    Self {
      data_handle,
      handle: Handle::from_raw_parts(0, 0),
      parent: None,
      children: Vec::new(),
    }
  }
}

impl<T> ArenaTree<T> {
  pub fn new(root_data: T) -> Self {
    todo!();
    // let mut tree = Self{
    //   nodes_data: Arena::new(),
    //   nodes: Arena::new(),
    // };
    // let root = tree.nodes_data.insert(root_data);
    // tree.nodes.insert(ArenaTreeNode::new(root));
    // tree
  }

  pub fn new_node() -> ArenaTreeNodeHandle<T> {
    todo!()
  }

  pub fn get_node(&self, handle: ArenaTreeNodeHandle<T>) -> &ArenaTreeNode<T> {
    self.nodes.get(handle).unwrap()
  }

  pub fn get_node_mut(&mut self, handle: ArenaTreeNodeHandle<T>) -> &mut ArenaTreeNode<T> {
    self.nodes.get_mut(handle).unwrap()
  }

  pub fn get_node_data(&self, handle: Handle<T>) -> &T {
    self.nodes_data.get(handle).unwrap()
  }

  pub fn get_node_data_mut(&mut self, handle: Handle<T>) -> &mut T {
    self.nodes_data.get_mut(handle).unwrap()
  }

  pub fn get_node_data_by_node(&self, handle: ArenaTreeNodeHandle<T>) -> &T {
    self
      .nodes_data
      .get(self.get_node(handle).data_handle)
      .unwrap()
  }

  pub fn get_node_data_mut_by_node(&mut self, handle: ArenaTreeNodeHandle<T>) -> &mut T {
    self
      .nodes_data
      .get_mut(self.get_node(handle).data_handle)
      .unwrap()
  }

  pub fn add_to_root(&mut self, child_id: ArenaTreeNodeHandle<T>) {
    todo!()
    // self.node_add_child_by_id(self.root, child_id);
  }

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
