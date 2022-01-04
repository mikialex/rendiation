use std::{cell::RefCell, rc::Rc};

use arena_tree::ArenaTree;
use rendiation_algebra::*;
use rendiation_controller::Transformed3DControllee;

use crate::ResourceWrapped;

use super::SceneNodeHandle;

pub type SceneNodeData = ResourceWrapped<SceneNodeDataImpl>;

pub struct SceneNodeDataImpl {
  pub visible: bool,
  pub local_matrix: Mat4<f32>,
  pub net_visible: bool,
  pub world_matrix: Mat4<f32>,
}

impl Default for SceneNodeDataImpl {
  fn default() -> Self {
    Self {
      visible: true,
      local_matrix: Mat4::one(),
      net_visible: true,
      world_matrix: Mat4::one(),
    }
  }
}

impl Transformed3DControllee for SceneNodeDataImpl {
  fn matrix(&self) -> &Mat4<f32> {
    &self.local_matrix
  }

  fn matrix_mut(&mut self) -> &mut Mat4<f32> {
    &mut self.local_matrix
  }
}

impl SceneNodeDataImpl {
  pub fn hierarchy_update(&mut self, parent: Option<&Self>) {
    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self.world_matrix = parent.world_matrix * self.local_matrix;
      }
    } else {
      self.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }
  }

  pub fn set_position(&mut self, position: (f32, f32, f32)) -> &mut Self {
    self.local_matrix = Mat4::translate(position.0, position.1, position.2); // todo
    self
  }
}

#[derive(Clone)]
struct SceneNodeRef {
  nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
  handle: SceneNodeHandle,
}

impl Drop for SceneNodeRef {
  fn drop(&mut self) {
    let mut nodes = self.nodes.borrow_mut();
    nodes.free_node(self.handle)
  }
}

pub struct SceneNodeInner {
  nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
  parent: Option<Rc<SceneNodeRef>>,
  inner: Rc<SceneNodeRef>,
}

impl SceneNodeInner {
  pub fn from_root(nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>) -> Self {
    let nodes_info = nodes.borrow();
    let root = SceneNodeRef {
      nodes: nodes.clone(),
      handle: nodes_info.root(),
    };
    Self {
      nodes: nodes.clone(),
      parent: None,
      inner: Rc::new(root),
    }
  }

  pub fn create_child(&self) -> Self {
    let mut nodes_info = self.nodes.borrow_mut();
    let handle = nodes_info.create_node(ResourceWrapped::new(SceneNodeDataImpl::default())); // todo use from
    let inner = SceneNodeRef {
      nodes: self.nodes.clone(),
      handle,
    };

    nodes_info.node_add_child_by_id(self.inner.handle, handle);

    Self {
      nodes: self.nodes.clone(),
      parent: Some(self.inner.clone()),
      inner: Rc::new(inner),
    }
  }

  pub fn mutate<F: FnMut(&mut SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let mut nodes = self.nodes.borrow_mut();
    let node = nodes.get_node_mut(self.inner.handle).data_mut();
    f(node)
  }

  pub fn visit<F: FnMut(&SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let nodes = self.nodes.borrow();
    let node = nodes.get_node(self.inner.handle).data();
    f(node)
  }
}

impl Drop for SceneNodeInner {
  fn drop(&mut self) {
    let mut nodes = self.nodes.borrow_mut();
    if let Some(parent) = self.parent.as_ref() {
      nodes.node_remove_child_by_id(parent.handle, self.inner.handle);
    }
  }
}

#[derive(Clone)]
pub struct SceneNode {
  inner: Rc<RefCell<SceneNodeInner>>,
}

impl SceneNode {
  pub fn from_root(nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>) -> Self {
    let inner = SceneNodeInner::from_root(nodes);
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }

  pub fn create_child(&self) -> Self {
    let inner = self.inner.borrow();
    let inner = inner.create_child();

    SceneNode {
      inner: Rc::new(RefCell::new(inner)),
    }
  }

  pub fn mutate<F: FnMut(&mut SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let inner = self.inner.borrow();
    let mut nodes = inner.nodes.borrow_mut();
    let node = nodes.get_node_mut(inner.inner.handle).data_mut();
    f(node)
  }

  pub fn visit<F: FnMut(&SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let inner = self.inner.borrow();
    let nodes = inner.nodes.borrow();
    let node = nodes.get_node(inner.inner.handle).data();
    f(node)
  }
}
