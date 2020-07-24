use crate::{RenderObjectHandle, SceneGraphBackend, Scene};
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

impl<T: SceneGraphBackend> Scene<T> {
  pub fn node_add_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle<T>,
    child_handle: SceneNodeHandle<T>,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.add(child);
  }

  pub fn node_remove_child_by_handle(
    &mut self,
    parent_handle: SceneNodeHandle<T>,
    child_handle: SceneNodeHandle<T>,
  ) {
    let (parent, child) = self
      .nodes
      .get_parent_child_pair(parent_handle, child_handle);
    parent.remove(child);
  }

  pub fn get_node(&self, handle: SceneNodeHandle<T>) -> &SceneNode<T> {
    self.nodes.get_node(handle)
  }

  pub fn get_node_mut(&mut self, handle: SceneNodeHandle<T>) -> &mut SceneNode<T> {
    self.nodes.get_node_mut(handle)
  }

  pub fn create_new_node(&mut self) -> &mut SceneNode<T> {
    let handle = self.nodes.create_node(SceneNodeData::new());
    self.nodes.get_node_mut(handle)
  }

  pub fn get_node_render_data(&self, handle: SceneNodeHandle<T>) -> &RenderData {
    &self.nodes.get_node(handle).data().render_data
  }

  pub fn free_node(&mut self, handle: SceneNodeHandle<T>) {
    self.nodes.free_node(handle);
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
