#![feature(type_alias_impl_trait)]

use std::sync::Arc;

use abst::TreeNodeMutPtr;
use incremental::IncrementalBase;
use parking_lot::RwLock;
pub use rendiation_abstract_tree::*;
use storage::*;

mod share;
pub use share::*;

mod reactive_impl;
pub use reactive_impl::*;

mod update;
pub use update::*;

mod update_full;
pub use update_full::*;

mod abst;
mod inc;
pub use inc::*;

#[cfg(test)]
mod test;

pub trait CoreTree {
  type Node;
  type Handle: Copy;

  fn try_recreate_handle(&self, index: usize) -> Option<Self::Handle>;
  fn recreate_handle(&self, index: usize) -> Self::Handle {
    self
      .try_recreate_handle(index)
      .expect("tree handle can not rebuild, maybe pair tree is corrupted")
  }

  fn node_has_parent(&self, handle: Self::Handle) -> bool;
  fn get_node_data(&self, handle: Self::Handle) -> &Self::Node;
  fn get_node_data_mut(&mut self, handle: Self::Handle) -> &mut Self::Node;

  fn create_node(&mut self, data: Self::Node) -> Self::Handle;
  fn delete_node(&mut self, handle: Self::Handle) -> Option<Self::Node>;
  fn node_add_child_by(
    &mut self,
    parent: Self::Handle,
    child_to_attach: Self::Handle,
  ) -> Result<(), TreeMutationError>;
  fn node_detach_parent(&mut self, child_to_detach: Self::Handle) -> Result<(), TreeMutationError>;
}

pub struct TreeCollection<T> {
  nodes: Arena<TreeNode<T>>,
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

impl<T> CoreTree for TreeCollection<T> {
  type Node = T;
  type Handle = TreeNodeHandle<T>;

  fn try_recreate_handle(&self, index: usize) -> Option<Self::Handle> {
    self.try_recreate_handle(index)
  }

  fn node_has_parent(&self, handle: Self::Handle) -> bool {
    self.nodes.get(handle).unwrap().parent.is_some()
  }

  fn create_node(&mut self, data: T) -> TreeNodeHandle<T> {
    self.nodes.insert_with(|handle| TreeNode {
      handle,
      parent: None,
      previous_sibling: None,
      next_sibling: None,
      first_child: None,
      data,
    })
  }

  fn delete_node(&mut self, handle: TreeNodeHandle<T>) -> Option<T> {
    self.nodes.remove(handle).map(|n| n.data)
  }

  fn get_node_data(&self, handle: TreeNodeHandle<T>) -> &T {
    self.get_node(handle).data()
  }

  fn get_node_data_mut(&mut self, handle: TreeNodeHandle<T>) -> &mut T {
    self.get_node_mut(handle).data_mut()
  }

  fn node_add_child_by(
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

  fn node_detach_parent(
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
}

impl<T> TreeCollection<T> {
  pub fn nodes(&self) -> &Arena<TreeNode<T>> {
    &self.nodes
  }

  pub fn capacity(&self) -> usize {
    self.nodes.capacity()
  }

  pub fn get_node(&self, handle: TreeNodeHandle<T>) -> &TreeNode<T> {
    self.nodes.get(handle).unwrap()
  }

  pub fn get_node_mut(&mut self, handle: TreeNodeHandle<T>) -> &mut TreeNode<T> {
    self.nodes.get_mut(handle).unwrap()
  }

  pub fn try_get_node(&self, handle: TreeNodeHandle<T>) -> Option<&TreeNode<T>> {
    self.nodes.get(handle)
  }

  pub fn try_get_node_mut(&mut self, handle: TreeNodeHandle<T>) -> Option<&mut TreeNode<T>> {
    self.nodes.get_mut(handle)
  }

  pub fn try_recreate_handle(&self, index: usize) -> Option<TreeNodeHandle<T>> {
    self.nodes.get_handle(index)
  }

  pub fn is_handle_valid(&self, handle: TreeNodeHandle<T>) -> bool {
    self.nodes.get(handle).is_some()
  }

  fn get_parent_child_pair(
    &mut self,
    parent: TreeNodeHandle<T>,
    child: TreeNodeHandle<T>,
  ) -> (&mut TreeNode<T>, &mut TreeNode<T>) {
    self.nodes.get_mut_pair((parent, child)).unwrap()
  }

  pub(crate) fn create_node_mut_ptr(&mut self, handle: TreeNodeHandle<T>) -> TreeNodeMutPtr<T> {
    let tree = self as *mut _;
    let node = self.get_node_mut(handle);
    TreeNodeMutPtr { tree, node }
  }

  pub fn traverse_mut_pair(
    &mut self,
    start: TreeNodeHandle<T>,
    mut visitor: impl FnMut(&mut TreeNode<T>, Option<&mut TreeNode<T>>) -> NextTraverseVisit,
  ) {
    self
      .create_node_mut_ptr(start)
      .traverse_pair_subtree_mut(&mut |child, parent| {
        let parent = parent.map(|parent| unsafe { &mut (*parent.node) });
        let child = unsafe { &mut (*child.node) };
        visitor(child, parent)
      });
  }

  pub fn traverse_mut(
    &mut self,
    start: TreeNodeHandle<T>,
    mut visitor: impl FnMut(&mut TreeNode<T>) -> NextTraverseVisit,
  ) {
    self
      .create_node_mut_ptr(start)
      .traverse_pair_subtree_mut(&mut |child, _| {
        let child = unsafe { &mut (*child.node) };
        visitor(child)
      });
  }

  pub fn iter_node_idx(&self) -> impl Iterator<Item = usize> + '_ {
    self.nodes.iter().map(|(h, _)| h.index())
  }
}
