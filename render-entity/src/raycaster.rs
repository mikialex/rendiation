use rendiation_math::Vec2;
use rendiation_math_entity::Ray;

pub trait Raycaster {
  fn generate_ray(&self, view_position: Vec2<f32>) -> Ray;
}

pub trait Raycastable<T> {
  fn get_first_hit(&self, ray: &Ray) -> T;
}
