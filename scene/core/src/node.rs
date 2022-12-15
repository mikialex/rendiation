use crate::*;

use tree::{ShareTreeNode, TreeCollection, TreeNodeHandle};

pub type SceneNodeData = Identity<SceneNodeDataImpl>;
pub type SceneNodeHandle = TreeNodeHandle<SceneNodeData>;

#[derive(Incremental)]
pub struct SceneNodeDataImpl {
  pub local_matrix: Mat4<f32>,
  pub world_matrix: Mat4<f32>,
  pub visible: bool,
  pub net_visible: bool,
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

impl SceneNodeDataImpl {
  pub fn world_matrix(&self) -> Mat4<f32> {
    self.world_matrix
  }

  pub fn net_visible(&self) -> bool {
    self.net_visible
  }
}

#[derive(Clone)]
pub struct SceneNode {
  inner: ShareTreeNode<SceneNodeData>,
}

clone_self_incremental!(SceneNode);

impl SceneNode {
  pub fn from_root(nodes: Arc<RwLock<TreeCollection<SceneNodeData>>>) -> Self {
    Self {
      inner: ShareTreeNode::create_new_root(nodes, Default::default()),
    }
  }

  pub fn raw_handle(&self) -> SceneNodeHandle {
    self.inner.raw_handle()
  }

  #[must_use]
  pub fn create_child(&self) -> Self {
    Self {
      inner: self.inner.create_child_default(),
    }
  }

  pub fn mutate<F: FnOnce(Mutating<SceneNodeDataImpl>) -> T, T>(&self, f: F) -> T {
    self.inner.mutate(|node| node.mutate(f))
  }

  pub fn visit<F: FnMut(&SceneNodeData) -> T, T>(&self, f: F) -> T {
    self.inner.visit(f)
  }

  pub fn visit_parent<F: FnMut(&SceneNodeData) -> T, T>(&self, f: F) -> Option<T> {
    self.inner.visit_parent(f)
  }

  pub fn set_local_matrix(&self, mat: Mat4<f32>) {
    self.mutate(|mut node| node.modify(SceneNodeDataImplDelta::local_matrix(mat)));
  }
  pub fn get_local_matrix(&self) -> Mat4<f32> {
    self.visit(|node| node.local_matrix)
  }

  pub fn set_visible(&self, visible: bool) {
    self.mutate(|mut node| node.modify(SceneNodeDataImplDelta::visible(visible)));
  }

  pub fn get_world_matrix(&self) -> Mat4<f32> {
    self.visit(|n| n.world_matrix)
  }
}
