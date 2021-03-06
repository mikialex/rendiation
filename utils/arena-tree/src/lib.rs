use arena::*;

pub struct ArenaTree<T> {
  nodes: Arena<ArenaTreeNode<T>>,
}
pub type ArenaTreeNodeHandle<T> = Handle<ArenaTreeNode<T>>;

pub struct ArenaTreeNode<T> {
  data: T,
  handle: ArenaTreeNodeHandle<T>,
  parent: Option<ArenaTreeNodeHandle<T>>,
  children: Vec<ArenaTreeNodeHandle<T>>,
}

impl<T> ArenaTreeNode<T> {
  pub fn data(&self) -> &T {
    &self.data
  }

  pub fn data_mut(&mut self) -> &mut T {
    &mut self.data
  }

  pub fn handle(&self) -> ArenaTreeNodeHandle<T> {
    self.handle
  }

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

  pub fn root(&self) -> ArenaTreeNodeHandle<T> {
    Handle::from_raw_parts(0, 0)
  }

  pub fn nodes(&self) -> &Arena<ArenaTreeNode<T>> {
    &self.nodes
  }

  pub fn create_node(&mut self, node_data: T) -> ArenaTreeNodeHandle<T> {
    let node = ArenaTreeNode::new(node_data);
    let handle = self.nodes.insert(node);
    self.nodes.get_mut(handle).unwrap().handle = handle;
    handle
  }

  pub fn free_node(&mut self, handle: ArenaTreeNodeHandle<T>) {
    self.nodes.remove(handle);
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

  pub fn get_parent_child_pair(
    &mut self,
    parent_id: ArenaTreeNodeHandle<T>,
    child_id: ArenaTreeNodeHandle<T>,
  ) -> (&mut ArenaTreeNode<T>, &mut ArenaTreeNode<T>) {
    let (parent, child) = self.nodes.get2_mut(parent_id, child_id);
    (parent.unwrap(), child.unwrap())
  }

  pub fn traverse_iter(&self, start: ArenaTreeNodeHandle<T>) -> TraverseIter<'_, T> {
    TraverseIter {
      tree: self,
      visit_stack: vec![start],
    }
  }

  pub fn traverse_parent(
    &mut self,
    index: ArenaTreeNodeHandle<T>,
    visitor: &mut impl FnMut(&mut ArenaTreeNode<T>, Option<&mut ArenaTreeNode<T>>),
  ) {
    if let Some(parent) = self.get_node(index).parent {
      self.traverse_parent(parent, visitor)
    }
    if let Some(parent_index) = self.get_node(index).parent {
      let (parent, this) = self.get_parent_child_pair(parent_index, index);
      visitor(this, Some(parent));
    } else {
      let this = self.get_node_mut(index);
      visitor(this, None);
    }
  }

  pub fn traverse_mut(
    &mut self,
    start_index: ArenaTreeNodeHandle<T>,
    visit_stack: &mut Vec<ArenaTreeNodeHandle<T>>,
    mut visitor: impl FnMut(&mut ArenaTreeNode<T>, Option<&mut ArenaTreeNode<T>>) -> NextTraverseVisit,
  ) {
    use NextTraverseVisit::*;
    visit_stack.clear();
    visit_stack.push(start_index);

    while let Some(index) = visit_stack.pop() {
      let (next, this) = if let Some(parent_index) = self.get_node(index).parent {
        let (parent, this) = self.get_parent_child_pair(parent_index, index);
        (visitor(this, Some(parent)), this)
      } else {
        let this = self.get_node_mut(index);
        (visitor(this, None), this)
      };

      match next {
        Exit => return,
        VisitChildren => visit_stack.extend(this.children.iter().cloned()),
        SkipChildren => continue,
      };
    }
  }
}

impl<T> ArenaTree<T> {
  pub fn traverse<'a>(
    &'a self,
    start_index: ArenaTreeNodeHandle<T>,
    visit_stack: &mut Vec<ArenaTreeNodeHandle<T>>,
    mut visitor: impl FnMut(&'a ArenaTreeNode<T>, Option<&'a ArenaTreeNode<T>>) -> NextTraverseVisit,
  ) {
    use NextTraverseVisit::*;
    visit_stack.clear();
    visit_stack.push(start_index);

    while let Some(index) = visit_stack.pop() {
      let (next, this) = if let Some(parent_index) = self.get_node(index).parent {
        let (parent, this) = (self.get_node(parent_index), self.get_node(index));
        (visitor(this, Some(parent)), this)
      } else {
        let this = self.get_node(index);
        (visitor(this, None), this)
      };

      match next {
        Exit => return,
        VisitChildren => visit_stack.extend(this.children.iter().cloned()),
        SkipChildren => continue,
      };
    }
  }
}

pub enum NextTraverseVisit {
  Exit,
  VisitChildren,
  SkipChildren,
}

pub struct TraverseIter<'a, T> {
  tree: &'a ArenaTree<T>,
  visit_stack: Vec<ArenaTreeNodeHandle<T>>,
}

impl<'a, T> Iterator for TraverseIter<'a, T> {
  type Item = (ArenaTreeNodeHandle<T>, &'a T);

  fn next(&mut self) -> Option<Self::Item> {
    self.visit_stack.pop().map(|handle| {
      let nodes = &self.tree;
      let node = nodes.get_node(handle);
      self.visit_stack.extend(node.children.iter().cloned());
      (handle, node.data())
    })
  }
}
