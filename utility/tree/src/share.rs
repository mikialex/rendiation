use crate::*;

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
  fn delete_node(&self, handle: Self::Handle) -> Option<Self::Node>;
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
    v(&self.read())
  }

  fn recreate_handle(&self, index: usize) -> Self::Handle {
    self.read().recreate_handle(index)
  }

  fn node_has_parent(&self, handle: Self::Handle) -> bool {
    self.read().node_has_parent(handle)
  }

  fn visit_node_data<R>(&self, handle: Self::Handle, v: impl FnOnce(&Self::Node) -> R) -> R {
    v(self.read().get_node_data(handle))
  }

  fn mutate_node_data<R>(&self, handle: Self::Handle, v: impl FnOnce(&mut Self::Node) -> R) -> R {
    v(self.write().get_node_data_mut(handle))
  }

  fn create_node(&self, data: Self::Node) -> Self::Handle {
    self.write().create_node(data)
  }

  fn delete_node(&self, handle: Self::Handle) -> Option<Self::Node> {
    self.write().delete_node(handle)
  }

  fn node_add_child_by(
    &self,
    parent: Self::Handle,
    child_to_attach: Self::Handle,
  ) -> Result<(), TreeMutationError> {
    self.write().node_add_child_by(parent, child_to_attach)
  }

  fn node_detach_parent(&self, child_to_detach: Self::Handle) -> Result<(), TreeMutationError> {
    self.write().node_detach_parent(child_to_detach)
  }
}

pub struct NodeRef<T: ShareCoreTree> {
  pub(crate) nodes: Arc<T>,
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
    self.nodes.delete_node(self.handle);
  }
}

pub struct NodeImpl<T: ShareCoreTree> {
  pub nodes: Arc<T>,
  parent: Option<ShareTreeNode<T>>,
  inner: Arc<NodeRef<T>>,
}

impl<T: ShareCoreTree> NodeImpl<T> {
  pub fn new(inner: NodeRef<T>) -> Self {
    Self {
      nodes: inner.nodes.clone(),
      parent: None,
      inner: Arc::new(inner),
    }
  }

  pub fn detach_from_parent(&mut self) -> Result<(), TreeMutationError> {
    self.nodes.node_detach_parent(self.inner.handle)
  }
}

impl<T: ShareCoreTree> Drop for NodeImpl<T> {
  fn drop(&mut self) {
    // the inner should check, but we add here for guard
    if self.parent.is_some() {
      self.detach_from_parent().ok();
    }
  }
}

pub struct ShareTreeNode<T: ShareCoreTree> {
  pub inner: Arc<RwLock<NodeImpl<T>>>,
}

impl<T: ShareCoreTree> Clone for ShareTreeNode<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: ShareCoreTree> ShareTreeNode<T> {
  pub fn new_as_root(n: T::Node, tree: &Arc<T>) -> Self {
    let root = tree.create_node(n);

    let root = NodeRef {
      nodes: tree.clone(),
      handle: root,
    };

    let root = NodeImpl::new(root);

    ShareTreeNode {
      inner: Arc::new(RwLock::new(root)),
    }
  }

  pub fn get_node_collection(&self) -> Arc<T> {
    self.inner.read().inner.nodes.clone()
  }

  pub fn raw_handle(&self) -> T::Handle {
    self.inner.read().inner.handle
  }

  pub fn parent(&self) -> Option<Self> {
    self.inner.read().parent.clone()
  }

  pub fn raw_handle_parent(&self) -> Option<T::Handle> {
    let inner = self.inner.read();
    inner.parent.as_ref().map(|p| p.raw_handle())
  }

  pub fn visit_raw_storage<F: FnOnce(&T) -> R, R>(&self, v: F) -> R {
    let inner = self.inner.read();
    v(&inner.nodes)
  }

  pub fn detach_from_parent(&self) -> Result<(), TreeMutationError> {
    self.inner.write().detach_from_parent()
  }

  pub fn attach_to(&self, parent: &Self) -> Result<(), TreeMutationError> {
    let mut inner = self.inner.write();

    inner
      .nodes
      .node_add_child_by(parent.raw_handle(), inner.inner.handle)?;

    inner.parent = Some(parent.clone());

    Ok(())
  }

  #[must_use]
  pub fn create_child(&self, n: T::Node) -> Self {
    let inner = self.inner.read();

    let child = NodeImpl::new(NodeRef {
      nodes: inner.nodes.clone(),
      handle: inner.nodes.create_node(n),
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
    let inner = self.inner.read();
    inner.nodes.mutate_node_data(inner.inner.handle, f)
  }

  pub fn visit<F: FnOnce(&T::Node) -> R, R>(&self, f: F) -> R {
    let inner = self.inner.read();
    inner.nodes.visit_node_data(inner.inner.handle, f)
  }

  pub fn visit_parent<F: FnOnce(&T::Node) -> R, R>(&self, f: F) -> Option<R> {
    let inner = self.inner.read();
    if let Some(parent) = &inner.parent {
      inner.nodes.visit_node_data(parent.raw_handle(), f).into()
    } else {
      None
    }
  }
}
