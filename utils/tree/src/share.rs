use std::sync::{Arc, RwLock};

use crate::CoreTree;

#[derive(Default)]
pub struct SharedTreeCollection<T> {
  pub inner: Arc<RwLock<T>>,
}

impl<T: CoreTree> SharedTreeCollection<T> {
  pub fn visit_inner<R>(&self, v: impl FnOnce(&T) -> R) -> R {
    let tree = self.inner.read().unwrap();
    v(&tree)
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

impl<T: CoreTree> NodeRef<T> {
  pub fn new_by_base(&self, nodes: &SharedTreeCollection<T>) -> Self {
    Self {
      nodes: nodes.clone(),
      handle: self.handle,
    }
  }
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

  pub fn new_by_base(&self, base: &SharedTreeCollection<T>) -> Self {
    Self {
      nodes: base.clone(),
      parent: self
        .parent
        .as_ref()
        .map(|parent| Arc::new(parent.new_by_base(base))),
      inner: Arc::new(self.inner.new_by_base(base)),
    }
  }

  pub fn map_handle(&mut self, mapper: impl Fn(T::Handle) -> T::Handle) {
    let new_inner_handle = mapper(self.inner.handle);
    self.inner = Arc::new(NodeRef {
      nodes: self.nodes.clone(),
      handle: new_inner_handle,
    });

    if let Some(parent) = &mut self.parent {
      let new_handle = mapper(parent.handle);
      *parent = Arc::new(NodeRef {
        nodes: self.nodes.clone(),
        handle: new_handle,
      });
    }
  }

  #[must_use]
  pub fn create_child(&self, n: T::Node) -> Self {
    let mut nodes_info = self.nodes.inner.write().unwrap();
    let handle = nodes_info.create_node(n);
    drop(nodes_info);
    let inner = NodeRef {
      nodes: self.nodes.clone(),
      handle,
    };

    let mut node = Self::create_new(inner);
    node.attach_to(self);
    node
  }

  pub fn attach_to(&mut self, parent: &Self) {
    let nodes = &mut self.nodes.inner.write().unwrap();
    nodes
      .node_add_child_by(parent.inner.handle, self.inner.handle)
      .unwrap();
    self.parent = Some(parent.inner.clone())
  }

  pub fn detach_from_parent(&mut self) {
    let nodes = &mut self.nodes.inner.write().unwrap();
    nodes.node_detach_parent(self.inner.handle).ok();
  }
}

impl<T: CoreTree> Drop for NodeInner<T> {
  fn drop(&mut self) {
    self.detach_from_parent()
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

  pub fn raw_handle_parent(&self) -> Option<T::Handle> {
    self.inner.read().unwrap().parent.as_ref().map(|p| p.handle)
  }

  pub fn new_by_base(&self, base: &SharedTreeCollection<T>) -> Self {
    let inner = self.inner.read().unwrap().new_by_base(base);
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  pub fn map_handle(&self, mapper: impl Fn(T::Handle) -> T::Handle) {
    let mut inner = self.inner.write().unwrap();
    inner.map_handle(mapper)
  }

  pub fn visit_raw_storage<F: FnOnce(&T) -> R, R>(&self, v: F) -> R {
    let inner = self.inner.read().unwrap();
    let tree = inner.nodes.inner.read().unwrap();
    v(&tree)
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
