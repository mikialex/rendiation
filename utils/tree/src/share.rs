use std::sync::{Arc, RwLock};

use crate::{CoreTree, TreeMutationError};

pub trait ShareCoreTree {
  type Node;
  type Handle: Copy;

  fn recreate_handle(&self, index: usize) -> Self::Handle;

  fn visit_node_data<R>(&self, handle: Self::Handle, v: impl FnOnce(&Self::Node) -> R) -> R;
  fn mutate_node_data<R>(&self, handle: Self::Handle, v: impl FnOnce(&mut Self::Node) -> R) -> R;

  fn create_node(&self, data: Self::Node) -> Self::Handle;
  fn delete_node(&self, handle: Self::Handle);
  fn node_add_child_by(
    &self,
    parent: Self::Handle,
    child_to_attach: Self::Handle,
  ) -> Result<(), TreeMutationError>;
  fn node_detach_parent(&self, child_to_detach: Self::Handle) -> Result<(), TreeMutationError>;
}

impl<T: CoreTree> ShareCoreTree for RwLock<T> {
  type Node = T::Node;

  type Handle = T::Handle;

  fn recreate_handle(&self, index: usize) -> Self::Handle {
    self.read().unwrap().recreate_handle(index)
  }

  fn visit_node_data<R>(&self, handle: Self::Handle, v: impl FnOnce(&Self::Node) -> R) -> R {
    v(self.read().unwrap().get_node_data(handle))
  }

  fn mutate_node_data<R>(&self, handle: Self::Handle, v: impl FnOnce(&mut Self::Node) -> R) -> R {
    v(self.write().unwrap().get_node_data_mut(handle))
  }

  fn create_node(&self, data: Self::Node) -> Self::Handle {
    self.write().unwrap().create_node(data)
  }

  fn delete_node(&self, handle: Self::Handle) {
    self.write().unwrap().delete_node(handle)
  }

  fn node_add_child_by(
    &self,
    parent: Self::Handle,
    child_to_attach: Self::Handle,
  ) -> Result<(), TreeMutationError> {
    self
      .write()
      .unwrap()
      .node_add_child_by(parent, child_to_attach)
  }

  fn node_detach_parent(&self, child_to_detach: Self::Handle) -> Result<(), TreeMutationError> {
    self.write().unwrap().node_detach_parent(child_to_detach)
  }
}

#[derive(Default)]
pub struct SharedTreeCollection<T> {
  pub(crate) inner: Arc<T>,
}

impl<T: ShareCoreTree> SharedTreeCollection<T> {
  pub fn inner(&self) -> &T {
    &self.inner
  }

  #[must_use]
  pub fn create_new_root(&self, n: T::Node) -> ShareTreeNode<T> {
    let root = self.inner.create_node(n);

    let root = NodeRef {
      nodes: self.clone(),
      handle: root,
    };

    let root = NodeInner::create_new(root);

    ShareTreeNode {
      inner: Arc::new(RwLock::new(root)),
    }
  }
}

impl<T> Clone for SharedTreeCollection<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct NodeRef<T: ShareCoreTree> {
  pub(crate) nodes: SharedTreeCollection<T>,
  pub(crate) handle: T::Handle,
}

impl<T: ShareCoreTree> Clone for NodeRef<T> {
  fn clone(&self) -> Self {
    Self {
      nodes: self.nodes.clone(),
      handle: self.handle,
    }
  }
}

impl<T: ShareCoreTree> Drop for NodeRef<T> {
  fn drop(&mut self) {
    self.nodes.inner.delete_node(self.handle)
  }
}

pub struct NodeInner<T: ShareCoreTree> {
  pub nodes: SharedTreeCollection<T>,
  parent: Option<Arc<NodeRef<T>>>,
  inner: Arc<NodeRef<T>>,
}

impl<T: ShareCoreTree> NodeInner<T> {
  pub fn create_new(inner: NodeRef<T>) -> Self {
    Self {
      nodes: inner.nodes.clone(),
      parent: None,
      inner: Arc::new(inner),
    }
  }

  #[must_use]
  pub fn create_child(&self, n: T::Node) -> Self {
    let handle = self.nodes.inner.create_node(n);
    let inner = NodeRef {
      nodes: self.nodes.clone(),
      handle,
    };

    let mut node = Self::create_new(inner);
    node.attach_to(self);
    node
  }

  pub fn attach_to(&mut self, parent: &Self) {
    self
      .nodes
      .inner
      .node_add_child_by(parent.inner.handle, self.inner.handle)
      .unwrap();
    self.parent = Some(parent.inner.clone())
  }

  pub fn detach_from_parent(&mut self) {
    self.nodes.inner.node_detach_parent(self.inner.handle).ok();
  }
}

impl<T: ShareCoreTree> Drop for NodeInner<T> {
  fn drop(&mut self) {
    // the inner should check, but we add here for guard
    if self.parent.is_some() {
      self.detach_from_parent()
    }
  }
}

pub struct ShareTreeNode<T: ShareCoreTree> {
  pub inner: Arc<RwLock<NodeInner<T>>>,
}

impl<T: ShareCoreTree> Clone for ShareTreeNode<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: ShareCoreTree> ShareTreeNode<T> {
  pub fn get_node_collection(&self) -> SharedTreeCollection<T> {
    self.inner.read().unwrap().inner.nodes.clone()
  }

  pub fn raw_handle(&self) -> T::Handle {
    self.inner.read().unwrap().inner.handle
  }

  pub fn raw_handle_parent(&self) -> Option<T::Handle> {
    self.inner.read().unwrap().parent.as_ref().map(|p| p.handle)
  }

  pub fn visit_raw_storage<F: FnOnce(&T) -> R, R>(&self, v: F) -> R {
    let inner = self.inner.read().unwrap();
    v(&inner.nodes.inner)
  }

  pub fn detach_from_parent(&self) {
    self.inner.write().unwrap().detach_from_parent()
  }

  pub fn attach_to(&self, parent: &Self) {
    self
      .inner
      .write()
      .unwrap()
      .attach_to(&parent.inner.read().unwrap());
  }

  #[must_use]
  pub fn create_child(&self, n: T::Node) -> Self {
    let inner = self.inner.read().unwrap();
    let inner = inner.create_child(n);

    ShareTreeNode {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  #[must_use]
  pub fn create_child_default(&self) -> Self
  where
    T::Node: Default,
  {
    self.create_child(T::Node::default())
  }

  pub fn mutate<F: FnOnce(&mut T::Node) -> R, R>(&self, f: F) -> R {
    let inner = self.inner.read().unwrap();
    inner.nodes.inner.mutate_node_data(inner.inner.handle, f)
  }

  pub fn visit<F: FnOnce(&T::Node) -> R, R>(&self, f: F) -> R {
    let inner = self.inner.read().unwrap();
    inner.nodes.inner.visit_node_data(inner.inner.handle, f)
  }

  pub fn visit_parent<F: FnOnce(&T::Node) -> R, R>(&self, f: F) -> Option<R> {
    let inner = self.inner.read().unwrap();
    if let Some(parent) = &inner.parent {
      inner.nodes.inner.visit_node_data(parent.handle, f).into()
    } else {
      None
    }
  }
}
