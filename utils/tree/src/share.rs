use std::sync::{Arc, RwLock};

use crate::{TreeCollection, TreeNodeHandle};

#[derive(Clone)]
struct NodeRef<T> {
  nodes: Arc<RwLock<TreeCollection<T>>>,
  handle: TreeNodeHandle<T>,
}

impl<T> Drop for NodeRef<T> {
  fn drop(&mut self) {
    let mut nodes = self.nodes.write().unwrap();
    nodes.delete_node(self.handle)
  }
}

struct NodeInner<T> {
  nodes: Arc<RwLock<TreeCollection<T>>>,
  parent: Option<Arc<NodeRef<T>>>,
  inner: Arc<NodeRef<T>>,
}

impl<T> NodeInner<T> {
  pub fn create_new(inner: NodeRef<T>) -> Self {
    Self {
      nodes: inner.nodes.clone(),
      parent: None,
      inner: Arc::new(inner),
    }
  }

  #[must_use]
  pub fn create_child(&self, n: T) -> Self {
    let nodes_info = &mut self.nodes.write().unwrap();
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

impl<T> Drop for NodeInner<T> {
  fn drop(&mut self) {
    let nodes = &mut self.nodes.write().unwrap();
    nodes.node_detach_parent(self.inner.handle).ok();
  }
}

pub struct ShareTreeNode<T> {
  inner: Arc<RwLock<NodeInner<T>>>,
}

impl<T> Clone for ShareTreeNode<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> ShareTreeNode<T> {
  pub fn raw_handle(&self) -> TreeNodeHandle<T> {
    self.inner.read().unwrap().inner.handle
  }

  #[must_use]
  pub fn create_new_root(nodes: Arc<RwLock<TreeCollection<T>>>, n: T) -> Self {
    let mut nodes_info = nodes.write().unwrap();

    let root = nodes_info.create_node(n);

    let root = NodeRef {
      nodes: nodes.clone(),
      handle: root,
    };

    let root = NodeInner::create_new(root);

    Self {
      inner: Arc::new(RwLock::new(root)),
    }
  }

  #[must_use]
  pub fn create_child(&self, n: T) -> Self {
    let inner = self.inner.read().unwrap();
    let inner = inner.create_child(n);

    ShareTreeNode {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  #[must_use]
  pub fn create_child_default(&self) -> Self
  where
    T: Default,
  {
    self.create_child(T::default())
  }

  pub fn mutate<F: FnMut(&mut T) -> R, R>(&self, mut f: F) -> R {
    let inner = self.inner.read().unwrap();
    let nodes = &mut inner.nodes.write().unwrap();
    let node = nodes.get_node_mut(inner.inner.handle).data_mut();
    f(node)
  }

  pub fn visit<F: FnMut(&T) -> R, R>(&self, mut f: F) -> R {
    let inner = self.inner.read().unwrap();
    let nodes = &inner.nodes.read().unwrap();
    let node = nodes.get_node(inner.inner.handle).data();
    f(node)
  }

  pub fn visit_parent<F: FnMut(&T) -> R, R>(&self, mut f: F) -> Option<R> {
    let inner = self.inner.read().unwrap();
    let nodes = &inner.nodes.read().unwrap();
    if let Some(parent) = &inner.parent {
      let node = nodes.get_node(parent.handle).data();
      f(node).into()
    } else {
      None
    }
  }
}
