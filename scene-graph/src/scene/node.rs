use crate::{RenderObjectHandle, SceneGraphBackend};
use arena::Handle;
use arena_tree::*;
use rendiation_math::{Mat4, One};
use rendiation_render_entity::BoundingData;

pub type SceneNodeHandle<T> = Handle<SceneNode<T>>;
pub type SceneNode<T> = ArenaTreeNode<SceneNodeData<T>>;

pub struct SceneNodeData<T: SceneGraphBackend> {
  pub render_objects: Vec<RenderObjectHandle<T>>,
  pub visible: bool,
  pub net_visible: bool,
  pub(crate) render_data: RenderData,
  pub local_matrix: Mat4<f32>,
}

impl<T: SceneGraphBackend> SceneNodeData<T> {
  pub(crate) fn new() -> Self {
    Self {
      render_objects: Vec::new(),
      visible: true,
      net_visible: true,
      render_data: RenderData::new(),
      local_matrix: Mat4::one(),
    }
  }

  pub fn add_render_object(&mut self, handle: RenderObjectHandle<T>) {
    self.render_objects.push(handle)
  }
}

pub struct RenderData {
  pub world_bounding: Option<BoundingData>,
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Mat4<f32>,
  pub camera_distance: f32,
}

impl RenderData {
  pub fn new() -> Self {
    Self {
      world_bounding: None,
      world_matrix: Mat4::one(),
      normal_matrix: Mat4::one(),
      camera_distance: 0.,
    }
  }
}
