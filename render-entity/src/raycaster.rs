use rendiation_math::Vec2;
use rendiation_math_entity::Ray3;

pub trait Raycaster {
  fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray3;
}
