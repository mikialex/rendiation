use abst::ArenaTreeNodeMutPtr;
pub use rendiation_abstract_tree::*;
use storage::{generational::Arena, *};

mod abst;

pub struct TreeCollection<T> {
  nodes: Storage<TreeNode<T>, Arena<TreeNode<T>>>,
}
pub type TreeNodeHandle<T> = Handle<TreeNode<T>, Arena<TreeNode<T>>>;

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

#[derive(Debug)]
pub enum TreeMutationError {
  DetachNoneParentNode,
  AttachNodeButHasParent,
}

impl<T> TreeCollection<T> {
  pub fn nodes(&self) -> &Storage<TreeNode<T>, Arena<TreeNode<T>>> {
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
    parent: TreeNodeHandle<T>,
    child: TreeNodeHandle<T>,
  ) -> (&mut TreeNode<T>, &mut TreeNode<T>) {
    self.nodes.get_mut_pair((parent, child)).unwrap()
  }

  pub fn node_add_child_by(
    &mut self,
    parent: TreeNodeHandle<T>,
    child_to_attach: TreeNodeHandle<T>,
  ) -> Result<(), TreeMutationError> {
    let old_first_child = {
      let (parent_node, child_node_to_attach) = self.get_parent_child_pair(parent, child_to_attach);

      if child_node_to_attach.parent.is_some() {
        return Err(TreeMutationError::AttachNodeButHasParent);
      }

      child_node_to_attach.parent = Some(parent);

      parent_node.first_child.replace(child_to_attach)
    };

    if let Some(old_first_child) = old_first_child {
      let (old_first_child_node, child_node_to_attach) = self
        .nodes
        .get_mut_pair((old_first_child, child_to_attach))
        .unwrap();

      old_first_child_node.previous_sibling = Some(child_to_attach);
      child_node_to_attach.next_sibling = Some(old_first_child)
    }

    Ok(())
  }

  pub fn node_detach_parent(
    &mut self,
    child_to_detach: TreeNodeHandle<T>,
  ) -> Result<(), TreeMutationError> {
    let child = self.get_node_mut(child_to_detach);

    // cleanup child's sib and parent relations:
    // if take parent failed, we safely early exist and keep tree states sound.
    let parent = child
      .parent
      .take()
      .ok_or(TreeMutationError::DetachNoneParentNode)?;
    let child_prev = child.previous_sibling.take();
    let child_next = child.next_sibling.take();

    if let Some(child_prev) = child_prev {
      // cleanup possible pre relation:
      let child_prev = self.get_node_mut(child_prev);
      child_prev.next_sibling = child_next;
    } else {
      // cleanup possible parent to first child relation:
      // means I'm the first child for parent
      let parent = self.get_node_mut(parent);
      parent.first_child = child_next;
    }

    // cleanup possible next relation:
    if let Some(child_next) = child_next {
      let child_next = self.get_node_mut(child_next);
      child_next.previous_sibling = child_prev;
    }

    Ok(())
  }

  // pub fn traverse_iter(&self, start: ArenaTreeNodeHandle<T>) -> TraverseIter<'_, T> {
  //   TraverseIter {
  //     tree: self,
  //     visit_stack: vec![start],
  //   }
  // }

  pub fn traverse_mut(
    &mut self,
    start: TreeNodeHandle<T>,
    mut visitor: impl FnMut(&mut TreeNode<T>, &mut TreeNode<T>) -> NextTraverseVisit,
  ) {
    let tree = self as *mut _;
    let node = self.get_node_mut(start);
    ArenaTreeNodeMutPtr { tree, node }.traverse_pair_mut(visitor);

    // use NextTraverseVisit::*;
    // visit_stack.clear();
    // visit_stack.push(start_index);

    // while let Some(index) = visit_stack.pop() {
    //   let (next, this) = if let Some(parent_index) = self.get_node(index).parent {
    //     let (parent, this) = self.get_parent_child_pair(parent_index, index);
    //     (visitor(this, Some(parent)), this)
    //   } else {
    //     let this = self.get_node_mut(index);
    //     (visitor(this, None), this)
    //   };

    //   match next {
    //     Exit => return,
    //     VisitChildren => visit_stack.extend(this.children.iter().cloned()),
    //     SkipChildren => continue,
    //   };
    // }
  }
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
