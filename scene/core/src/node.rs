use crate::*;

use tree::{ShareTreeNode, SharedTreeCollection, TreeCollection, TreeNodeHandle};

pub type SceneNodeData = Identity<SceneNodeDataImpl>;
pub type SceneNodeHandle = TreeNodeHandle<SceneNodeData>;

#[derive(Incremental, Clone)]
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
  inner: ShareTreeNode<TreeCollection<SceneNodeData>>,
}

clone_self_incremental!(SceneNode);

impl SceneNode {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(Partial<SceneNodeDataImpl>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    self.visit(|node| node.listen_by(mapper))
  }

  pub fn from_root(nodes: SharedTreeCollection<TreeCollection<SceneNodeData>>) -> Self {
    Self {
      inner: nodes.create_new_root(Default::default()),
    }
  }

  pub fn id(&self) -> usize {
    self.inner.visit(|n| n.id())
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

  pub fn visit<F: FnOnce(&SceneNodeData) -> T, T>(&self, f: F) -> T {
    self.inner.visit(f)
  }

  pub fn visit_parent<F: FnOnce(&SceneNodeData) -> T, T>(&self, f: F) -> Option<T> {
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
