use std::sync::{Arc, RwLock};

use crate::{CoreTree, TreeMutationError};

// todo add a trait extract the common part of core tree and share core tree
pub trait ShareCoreTree {
  type Node;
  type Handle: Copy;
  type Core: CoreTree;

  fn visit_core_tree<R>(&self, v: impl FnOnce(&Self::Core) -> R) -> R;

  fn recreate_handle(&self, index: usize) -> Self::Handle;

  fn node_has_parent(&self, handle: Self::Handle) -> bool;

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
  type Core = T;

  fn visit_core_tree<R>(&self, v: impl FnOnce(&Self::Core) -> R) -> R {
    v(&self.read().unwrap())
  }

  fn recreate_handle(&self, index: usize) -> Self::Handle {
    self.read().unwrap().recreate_handle(index)
  }

  fn node_has_parent(&self, handle: Self::Handle) -> bool {
    self.read().unwrap().node_has_parent(handle)
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

    let root = NodeInner::new(root);

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
  parent: Option<ShareTreeNode<T>>,
  inner: Arc<NodeRef<T>>,
}

impl<T: ShareCoreTree> NodeInner<T> {
  pub fn new(inner: NodeRef<T>) -> Self {
    Self {
      nodes: inner.nodes.clone(),
      parent: None,
      inner: Arc::new(inner),
    }
  }

  pub fn detach_from_parent(&mut self) -> Result<(), TreeMutationError> {
    self.nodes.inner.node_detach_parent(self.inner.handle)
  }
}

impl<T: ShareCoreTree> Drop for NodeInner<T> {
  fn drop(&mut self) {
    // the inner should check, but we add here for guard
    if self.parent.is_some() {
      self.detach_from_parent().ok();
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

  pub fn parent(&self) -> Option<Self> {
    self.inner.read().unwrap().parent.clone()
  }

  pub fn raw_handle_parent(&self) -> Option<T::Handle> {
    let inner = self.inner.read().unwrap();
    inner.parent.as_ref().map(|p| p.raw_handle())
  }

  pub fn visit_raw_storage<F: FnOnce(&T) -> R, R>(&self, v: F) -> R {
    let inner = self.inner.read().unwrap();
    v(&inner.nodes.inner)
  }

  pub fn detach_from_parent(&self) -> Result<(), TreeMutationError> {
    self.inner.write().unwrap().detach_from_parent()
  }

  pub fn attach_to(&self, parent: &Self) -> Result<(), TreeMutationError> {
    let mut inner = self.inner.write().unwrap();

    inner
      .nodes
      .inner
      .node_add_child_by(parent.raw_handle(), inner.inner.handle)?;

    inner.parent = Some(parent.clone());

    Ok(())
  }

  #[must_use]
  pub fn create_child(&self, n: T::Node) -> Self {
    let inner = self.inner.read().unwrap();

    let child = NodeInner::new(NodeRef {
      nodes: inner.nodes.clone(),
      handle: inner.nodes.inner.create_node(n),
    });

    let child = ShareTreeNode {
      inner: Arc::new(RwLock::new(child)),
    };

    child.attach_to(self).ok();

    child
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
      inner
        .nodes
        .inner
        .visit_node_data(parent.raw_handle(), f)
        .into()
    } else {
      None
    }
  }
}
