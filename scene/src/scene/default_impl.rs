use crate::{DrawcallHandle, SceneBackend, SceneNodeDataTrait};
use rendiation_algebra::*;
use rendiation_ral::{ResourceManager, UniformHandle, RAL};
use rendiation_render_entity::{BoundingInfo, Camera};

pub struct DefaultSceneBackend;

impl<T: RAL> SceneBackend<T> for DefaultSceneBackend {
  type NodeData = SceneNodeData<T>;
  type SceneData = ();
  fn create_node_data(resource: &mut ResourceManager<T>) -> Self::NodeData {
    SceneNodeData::new(resource)
  }
  fn free_node_data(node: &Self::NodeData, resource: &mut ResourceManager<T>) {
    resource
      .bindable
      .uniform_buffers
      .delete(node.render_data.matrix_data)
  }
}

pub struct SceneNodeData<T: RAL> {
  pub drawcalls: Vec<DrawcallHandle<T>>,
  pub visible: bool,
  pub net_visible: bool,
  pub render_data: RenderData<T>,
  pub local_matrix: Mat4<f32>,
  world_matrix: Mat4<f32>,
}

impl<T: RAL> SceneNodeDataTrait<T> for SceneNodeData<T> {
  type DrawcallIntoIterType = Vec<DrawcallHandle<T>>;
  fn update(
    &mut self,
    parent: Option<&Self>,
    camera: &Camera,
    resource: &mut ResourceManager<T>,
  ) -> bool {
    let mut self_matrix = resource
      .bindable
      .uniform_buffers
      .mutate(self.render_data.matrix_data);

    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self_matrix.world_matrix = parent.world_matrix * self.local_matrix;
        self.world_matrix = self_matrix.world_matrix;
        self_matrix.model_view_matrix = camera.matrix_inverse * self_matrix.world_matrix;
        self_matrix.normal_matrix = self_matrix.model_view_matrix.to_normal_matrix();
      }
    } else {
      self_matrix.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }

    self.net_visible
  }
  fn provide_drawcall(&self) -> &Self::DrawcallIntoIterType {
    &self.drawcalls
  }
}

impl<T: RAL> SceneNodeData<T> {
  pub fn new(resource: &mut ResourceManager<T>) -> Self {
    Self {
      drawcalls: Vec::new(),
      visible: true,
      net_visible: true,
      render_data: RenderData::new(resource),
      local_matrix: Mat4::one(),
      world_matrix: Mat4::one(),
    }
  }

  pub fn append_drawcall(&mut self, handle: DrawcallHandle<T>) {
    self.drawcalls.push(handle)
  }
}

pub struct RenderData<T: RAL> {
  pub world_bounding: Option<BoundingInfo>,
  pub matrix_data: UniformHandle<T, SceneNodeRenderMatrixData>,
  pub camera_distance: f32,
}

impl<T: RAL> RenderData<T> {
  pub fn new(resource: &mut ResourceManager<T>) -> Self {
    Self {
      world_bounding: None,
      matrix_data: resource
        .bindable
        .uniform_buffers
        .add(SceneNodeRenderMatrixData::default()),
      camera_distance: 0.,
    }
  }
}

pub use rendiation_derives::UniformBuffer;
#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct SceneNodeRenderMatrixData {
  pub model_view_matrix: Mat4<f32>,
  pub world_matrix: Mat4<f32>,
  pub normal_matrix: Mat3<f32>,
}

impl Default for SceneNodeRenderMatrixData {
  fn default() -> Self {
    Self {
      model_view_matrix: Mat4::one(),
      world_matrix: Mat4::one(),
      normal_matrix: Mat3::one(),
    }
  }
}
