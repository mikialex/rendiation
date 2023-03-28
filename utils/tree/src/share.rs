use std::sync::{Arc, RwLock};

use crate::CoreTree;

#[derive(Default)]
pub struct SharedTreeCollection<T> {
  pub(crate) inner: Arc<RwLock<T>>,
}

impl<T: CoreTree> SharedTreeCollection<T> {
  pub fn visit_inner(&self, v: impl FnOnce(&T)) {
    let tree = self.inner.read().unwrap();
    v(&tree);
  }

  #[must_use]
  pub fn create_new_root(&self, n: T::Node) -> ShareTreeNode<T> {
    let mut nodes_info = self.inner.write().unwrap();

    let root = nodes_info.create_node(n);

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

pub struct NodeRef<T: CoreTree> {
  pub(crate) nodes: SharedTreeCollection<T>,
  pub(crate) handle: T::Handle,
}

impl<T: CoreTree> Clone for NodeRef<T> {
  fn clone(&self) -> Self {
    Self {
      nodes: self.nodes.clone(),
      handle: self.handle,
    }
  }
}

impl<T: CoreTree> Drop for NodeRef<T> {
  fn drop(&mut self) {
    let mut nodes = self.nodes.inner.write().unwrap();
    nodes.delete_node(self.handle)
  }
}

struct NodeInner<T: CoreTree> {
  nodes: SharedTreeCollection<T>,
  parent: Option<Arc<NodeRef<T>>>,
  inner: Arc<NodeRef<T>>,
}

impl<T: CoreTree> NodeInner<T> {
  pub fn create_new(inner: NodeRef<T>) -> Self {
    Self {
      nodes: inner.nodes.clone(),
      parent: None,
      inner: Arc::new(inner),
    }
  }

  #[must_use]
  pub fn create_child(&self, n: T::Node) -> Self {
    let nodes_info = &mut self.nodes.inner.write().unwrap();
    let handle = nodes_info.create_node(n);
    let inner = NodeRef {
      nodes: self.nodes.clone(),
      handle,
    };

    nodes_info
      .node_add_child_by(self.inner.handle, handle)
      .unwrap();

    Self {
      nodes: self.nodes.clone(),
      parent: Some(self.inner.clone()),
      inner: Arc::new(inner),
    }
  }
}

impl<T: CoreTree> Drop for NodeInner<T> {
  fn drop(&mut self) {
    let nodes = &mut self.nodes.inner.write().unwrap();
    nodes.node_detach_parent(self.inner.handle).ok();
  }
}

pub struct ShareTreeNode<T: CoreTree> {
  inner: Arc<RwLock<NodeInner<T>>>,
}

impl<T: CoreTree> Clone for ShareTreeNode<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: CoreTree> ShareTreeNode<T> {
  pub fn raw_handle(&self) -> T::Handle {
    self.inner.read().unwrap().inner.handle
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
    let nodes = &mut inner.nodes.inner.write().unwrap();
    let node = nodes.get_node_data_mut(inner.inner.handle);
    f(node)
  }

  pub fn visit<F: FnOnce(&T::Node) -> R, R>(&self, f: F) -> R {
    let inner = self.inner.read().unwrap();
    let nodes = &inner.nodes.inner.read().unwrap();
    let node = nodes.get_node_data(inner.inner.handle);
    f(node)
  }

  pub fn visit_parent<F: FnOnce(&T::Node) -> R, R>(&self, f: F) -> Option<R> {
    let inner = self.inner.read().unwrap();
    let nodes = &inner.nodes.inner.read().unwrap();
    if let Some(parent) = &inner.parent {
      let node = nodes.get_node_data(parent.handle);
      f(node).into()
    } else {
      None
    }
  }
}
