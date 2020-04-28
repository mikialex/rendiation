use generational_arena::Index;
use rendiation_math::{Mat4, One};
use rendiation_render_entity::{BoundingData};
use super::scene::Scene;

pub struct SceneNode {
  transform_dirty_id: usize,
  self_id: Index,
  parent: Option<Index>,
  children: Vec<Index>
}

impl SceneNode {
  pub(crate) fn new() -> Self {
    Self{
      transform_dirty_id: 0,
      self_id: Index::from_raw_parts(0, 0), // later 
      parent: None,
      children: Vec::new()
    }
  }

  pub(crate) fn set_self_id(&mut self, id: Index) {
    self.self_id = id;
  }

  pub fn traverse(scene: &mut Scene, visitor: impl FnMut(&RenderObject)){
    
  }
}

pub trait TransformLocalWorld{
  fn get_local_transform();
  fn set_local_transform();
  fn get_world_transform();
  fn set_world_transform();
}

pub struct RenderObject{
  shading_index: Index,
  geometry_index: Index,
}

pub struct RenderData {
  world_bounding: Option<BoundingData>,
  world_matrix: Mat4<f32>,
  local_matrix: Mat4<f32>,
  normal_matrix: Mat4<f32>,
}

impl RenderData{
  pub fn new() -> Self{
    Self{
      world_bounding: None,
      world_matrix: Mat4::one(),
      local_matrix: Mat4::one(),
      normal_matrix: Mat4::one(),
    }
  }
}

pub struct ResourceManager{
  // shadings
  // geometries

}