use rendiation_math_entity::Ray;
use rendiation_math::Vec2;

pub trait Raycaster{
  fn generate_ray(&self, view_position: Vec2<f32>) -> Ray;
}

