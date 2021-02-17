use rendiation_algebra::Vec2;
use rendiation_geometry::Ray3;

pub trait Raycaster {
  fn create_screen_ray(&self, view_position: Vec2<f32>) -> Ray3;
}
