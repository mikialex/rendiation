use crate::*;

use arena_tree::{ArenaTree, ArenaTreeNodeHandle};
use rendiation_algebra::*;
use rendiation_controller::Transformed3DControllee;

pub type SceneNodeData = Identity<SceneNodeDataImpl>;
pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNodeData>;

pub struct SceneNodeDataImpl {
  pub visible: bool,
  pub local_matrix: Mat4<f32>,
  pub net_visible: bool,
  pub world_matrix: Mat4<f32>,
}

impl AsRef<Self> for SceneNodeDataImpl {
  fn as_ref(&self) -> &Self {
    self
  }
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
  nodes: Arc<RwLock<ArenaTree<SceneNodeData>>>,
  handle: SceneNodeHandle,
}

impl Drop for SceneNodeRef {
  fn drop(&mut self) {
    let mut nodes = self.nodes.write().unwrap();
    nodes.free_node(self.handle)
  }
}

pub struct SceneNodeInner {
  nodes: Arc<RwLock<ArenaTree<SceneNodeData>>>,
  parent: Option<Arc<SceneNodeRef>>,
  inner: Arc<SceneNodeRef>,
}

impl SceneNodeInner {
  pub fn from_root(nodes: Arc<RwLock<ArenaTree<SceneNodeData>>>) -> Self {
    let nodes_info = nodes.read().unwrap();
    let root = SceneNodeRef {
      nodes: nodes.clone(),
      handle: nodes_info.root(),
    };
    Self {
      nodes: nodes.clone(),
      parent: None,
      inner: Arc::new(root),
    }
  }

  #[must_use]
  pub fn create_child(&self) -> Self {
    let mut nodes_info = self.nodes.write().unwrap();
    let handle = nodes_info.create_node(Identity::new(SceneNodeDataImpl::default())); // todo use from
    let inner = SceneNodeRef {
      nodes: self.nodes.clone(),
      handle,
    };

    nodes_info.node_add_child_by_id(self.inner.handle, handle);

    Self {
      nodes: self.nodes.clone(),
      parent: Some(self.inner.clone()),
      inner: Arc::new(inner),
    }
  }

  pub fn mutate<F: FnMut(&mut SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let mut nodes = self.nodes.write().unwrap();
    let node = nodes.get_node_mut(self.inner.handle).data_mut();
    f(node)
  }

  pub fn visit<F: FnMut(&SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let nodes = self.nodes.read().unwrap();
    let node = nodes.get_node(self.inner.handle).data();
    f(node)
  }
}

impl Drop for SceneNodeInner {
  fn drop(&mut self) {
    let mut nodes = self.nodes.write().unwrap();
    if let Some(parent) = self.parent.as_ref() {
      nodes.node_remove_child_by_id(parent.handle, self.inner.handle);
    }
  }
}

#[derive(Clone)]
pub struct SceneNode {
  inner: Arc<RwLock<SceneNodeInner>>,
}

impl SceneNode {
  pub fn from_root(nodes: Arc<RwLock<ArenaTree<SceneNodeData>>>) -> Self {
    let inner = SceneNodeInner::from_root(nodes);
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  #[must_use]
  pub fn create_child(&self) -> Self {
    let inner = self.inner.read().unwrap();
    let inner = inner.create_child();

    SceneNode {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  pub fn mutate<F: FnMut(&mut SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let inner = self.inner.read().unwrap();
    let mut nodes = inner.nodes.write().unwrap();
    let node = nodes.get_node_mut(inner.inner.handle).data_mut();
    f(node)
  }

  pub fn visit<F: FnMut(&SceneNodeData) -> T, T>(&self, mut f: F) -> T {
    let inner = self.inner.read().unwrap();
    let nodes = inner.nodes.read().unwrap();
    let node = nodes.get_node(inner.inner.handle).data();
    f(node)
  }

  pub fn visit_parent<F: FnMut(&SceneNodeData) -> T, T>(&self, mut f: F) -> Option<T> {
    let inner = self.inner.read().unwrap();
    let nodes = inner.nodes.read().unwrap();
    if let Some(parent) = &inner.parent {
      let node = nodes.get_node(parent.handle).data();
      f(node).into()
    } else {
      None
    }
  }

  pub fn set_local_matrix(&self, mat: Mat4<f32>) {
    self.mutate(|node| node.local_matrix = mat);
  }
  pub fn get_local_matrix(&self) -> Mat4<f32> {
    self.visit(|node| node.local_matrix)
  }

  pub fn set_visible(&self, visible: bool) {
    self.mutate(|node| node.visible = visible);
  }

  pub fn get_world_matrix(&self) -> Mat4<f32> {
    self.visit(|n| n.world_matrix)
  }
}
