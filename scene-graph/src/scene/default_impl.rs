use crate::{
  RenderObjectHandle, SceneBackend, SceneNodeDataRenderObjectsProvider, SceneNodeDataTrait,
};
use rendiation_math::*;
use rendiation_ral::{RALBackend, RenderObject, ResourceManager};
use rendiation_render_entity::BoundingData;

pub struct DefaultSceneBackend;

impl<T: RALBackend> SceneBackend<T> for DefaultSceneBackend {
  type NodeData = SceneNodeData<T>;
  type SceneData = ();
}

pub struct SceneNodeData<T: RALBackend> {
  pub render_objects: Vec<RenderObjectHandle<T>>,
  pub visible: bool,
  pub net_visible: bool,
  pub render_data: RenderData,
  pub local_matrix: Mat4<f32>,
}

impl<T: RALBackend> Default for SceneNodeData<T> {
  fn default() -> Self {
    Self::new()
  }
}

// impl<'a, T: RALBackend> IntoIterator for &'a SceneNodeData<T> {
//   type Item = RenderObjectHandle<T>;
//   // type IntoIter = std::slice::Iter<'a, T, Item = RenderObjectHandle<T>>;

//   fn into_iter(self) -> Self::IntoIter {
//     self.render_objects.iter()
//   }
// }

impl<T: RALBackend> SceneNodeDataTrait<T> for SceneNodeData<T> {
  type RenderObjectIntoIterType = Vec<RenderObjectHandle<T>>;
  fn update_by_parent(&mut self, parent: Option<&Self>, resource: &mut ResourceManager<T>) -> bool {
    if let Some(parent) = parent {
      self.render_data.world_matrix = parent.render_data.world_matrix * self.local_matrix;
      self.net_visible = self.visible && parent.net_visible;
    }

    todo!()
  }
  fn provide_render_object(&self) -> &Self::RenderObjectIntoIterType {
    &self.render_objects
  }
}

impl<T: RALBackend> SceneNodeData<T> {
  pub fn new() -> Self {
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
