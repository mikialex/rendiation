use crate::{Camera, Scene};
use arena_tree::ArenaTreeNode;
use rendiation_algebra::{Mat3, Mat4};

pub type SceneNodeHandle = ArenaTreeNode<SceneNode>;

pub struct SceneNode {
  pub visible: bool,
  pub(crate) net_visible: bool,
  pub local_matrix: Mat4<f32>,
  pub(crate) world_matrix: Mat4<f32>,
  pub payload: Vec<SceneNodePayload>,
}

pub struct SceneNodeCameraRenderInfo {
  pub model_view_matrix: Mat4<f32>,
  pub normal_matrix: Mat3<f32>,
}

impl SceneNode {
  fn update(
    &mut self,
    parent: Option<&Self>,
    camera: &Camera,
    info: &mut SceneNodeCameraRenderInfo,
  ) -> bool {
    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self.world_matrix = parent.world_matrix * self.local_matrix;
        self.world_matrix = self.world_matrix;
        info.model_view_matrix = camera.matrix_inverse * self.world_matrix;
        info.normal_matrix = info.model_view_matrix.to_normal_matrix();
      }
    } else {
      self.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }

    self.net_visible
  }
}

pub enum SceneNodePayload {
  Drawable(Drawable),
}

pub struct Drawable {}

impl Scene {
  // pub fn get_root(&self) -> &SceneNode {
  //   self.nodes.get_node(self.nodes.root())
  // }

  // pub fn get_root_node_mut(&mut self) -> &mut SceneNode {
  //   self.get_node_mut(self.nodes.root())
  // }

  // pub fn add_to_scene_root(&mut self, child_handle: SceneNodeHandle) {
  //   self.node_add_child_by_handle(self.nodes.root(), child_handle);
  // }

  // pub fn node_add_child_by_handle(
  //   &mut self,
  //   parent_handle: SceneNodeHandle,
  //   child_handle: SceneNodeHandle,
  // ) {
  //   let (parent, child) = self
  //     .nodes
  //     .get_parent_child_pair(parent_handle, child_handle);
  //   parent.add(child);
  // }

  // pub fn node_remove_child_by_handle(
  //   &mut self,
  //   parent_handle: SceneNodeHandle,
  //   child_handle: SceneNodeHandle,
  // ) {
  //   let (parent, child) = self
  //     .nodes
  //     .get_parent_child_pair(parent_handle, child_handle);
  //   parent.remove(child);
  // }

  // pub fn get_node(&self, handle: SceneNodeHandle) -> &SceneNode {
  //   self.nodes.get_node(handle)
  // }

  // pub fn get_node_mut(&mut self, handle: SceneNodeHandle) -> &mut SceneNode {
  //   self.nodes.get_node_mut(handle)
  // }

  // pub fn create_new_node(&mut self) -> &mut SceneNode {
  //   let node = SceneNode::new();
  //   let handle = self.nodes.create_node(node);
  //   self.nodes.get_node_mut(handle)
  // }

  // pub fn free_node(&mut self, handle: SceneNodeHandle) {
  //   self.nodes.free_node(handle);
  // }
}
