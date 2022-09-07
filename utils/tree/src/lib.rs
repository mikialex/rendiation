pub use rendiation_abstract_tree::*;
use storage::{generational::GenerationalVecStorage, *};

mod abst;

pub struct TreeCollection<T> {
  nodes: Storage<TreeNode<T>, GenerationalVecStorage>,
}
pub type TreeNodeHandle<T> = Handle<TreeNode<T>, GenerationalVecStorage>;

pub struct TreeNode<T> {
  handle: TreeNodeHandle<T>,
  parent: Option<TreeNodeHandle<T>>,
  previous_sibling: Option<TreeNodeHandle<T>>,
  next_sibling: Option<TreeNodeHandle<T>>,
  first_child: Option<TreeNodeHandle<T>>,
  data: T,
}

impl<T> Default for TreeCollection<T> {
  fn default() -> Self {
    Self {
      nodes: Default::default(),
    }
  }
}

impl<T> TreeNode<T> {
  pub fn data(&self) -> &T {
    &self.data
  }

  pub fn data_mut(&mut self) -> &mut T {
    &mut self.data
  }

  pub fn handle(&self) -> TreeNodeHandle<T> {
    self.handle
  }
}

impl<T> TreeCollection<T> {
  pub fn nodes(&self) -> &Storage<TreeNode<T>, GenerationalVecStorage> {
    &self.nodes
  }

  pub fn create_node(&mut self, data: T) -> TreeNodeHandle<T> {
    self.nodes.insert_with(|handle| TreeNode {
      handle,
      parent: None,
      previous_sibling: None,
      next_sibling: None,
      first_child: None,
      data,
    })
  }

  pub fn delete_node(&mut self, handle: TreeNodeHandle<T>) {
    self.nodes.remove(handle);
  }

  pub fn get_node(&self, handle: TreeNodeHandle<T>) -> &TreeNode<T> {
    self.nodes.get(handle).unwrap()
  }

  pub fn get_node_mut(&mut self, handle: TreeNodeHandle<T>) -> &mut TreeNode<T> {
    self.nodes.get_mut(handle).unwrap()
  }

  pub fn get_parent_child_pair(
    &mut self,
    parent_id: TreeNodeHandle<T>,
    child_id: TreeNodeHandle<T>,
  ) -> (&mut TreeNode<T>, &mut TreeNode<T>) {
    self.nodes.get_mut_pair((parent_id, child_id)).unwrap()
  }

  pub fn node_add_child_by_id(
    &mut self,
    parent_id: TreeNodeHandle<T>,
    child_id: TreeNodeHandle<T>,
  ) {
    let (parent, child) = self.get_parent_child_pair(parent_id, child_id);
    todo!()
  }

  pub fn node_remove_child_by_id(
    &mut self,
    parent_id: TreeNodeHandle<T>,
    child_id: TreeNodeHandle<T>,
  ) {
    let (parent, child) = self.get_parent_child_pair(parent_id, child_id);
    todo!()
  }

  // pub fn traverse_iter(&self, start: ArenaTreeNodeHandle<T>) -> TraverseIter<'_, T> {
  //   TraverseIter {
  //     tree: self,
  //     visit_stack: vec![start],
  //   }
  // }

  // pub fn traverse_mut(
  //   &mut self,
  //   start_index: ArenaTreeNodeHandle<T>,
  //   visit_stack: &mut Vec<ArenaTreeNodeHandle<T>>,
  //   mut visitor: impl FnMut(&mut ArenaTreeNode<T>, Option<&mut ArenaTreeNode<T>>) -> NextTraverseVisit,
  // ) {
  //   use NextTraverseVisit::*;
  //   visit_stack.clear();
  //   visit_stack.push(start_index);

  //   while let Some(index) = visit_stack.pop() {
  //     let (next, this) = if let Some(parent_index) = self.get_node(index).parent {
  //       let (parent, this) = self.get_parent_child_pair(parent_index, index);
  //       (visitor(this, Some(parent)), this)
  //     } else {
  //       let this = self.get_node_mut(index);
  //       (visitor(this, None), this)
  //     };

  //     match next {
  //       Exit => return,
  //       VisitChildren => visit_stack.extend(this.children.iter().cloned()),
  //       SkipChildren => continue,
  //     };
  //   }
  // }
}

// pub struct TraverseIter<'a, T> {
//   tree: &'a ArenaTree<T>,
//   visit_stack: Vec<ArenaTreeNodeHandle<T>>,
// }

// impl<'a, T> Iterator for TraverseIter<'a, T> {
//   type Item = (ArenaTreeNodeHandle<T>, &'a T);

//   fn next(&mut self) -> Option<Self::Item> {
//     self.visit_stack.pop().map(|handle| {
//       let nodes = &self.tree;
//       let node = nodes.get_node(handle);
//       self.visit_stack.extend(node.children.iter().cloned());
//       (handle, node.data())
//     })
//   }
// }
