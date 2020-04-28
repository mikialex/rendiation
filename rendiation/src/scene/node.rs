use generational_arena::Index;
use rendiation_math::Mat4;
use rendiation_render_entity::{BoundingData};
use super::scene::Scene;

pub struct SceneNode {
  transform_dirty_id: usize,
  self_id: Index,
  parent: Option<Index>,
  children: Vec<Index>
}

impl SceneNode {
  fn traverse(scene: &mut Scene, visitor: impl FnMut(&RenderObject)){
    
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
  world_bounding: BoundingData,
  world_matrix: Mat4<f32>,
  local_matrix: Mat4<f32>,
  normal_matrix: Mat4<f32>,
}

pub struct ResourceManager{
  // shadings
  // geometries

}