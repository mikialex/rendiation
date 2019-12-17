
use rendiation_math::*;
use rendiation_math::vec::Math;

#[derive(Debug, Copy, Clone)]
pub struct Box3 {
  pub min: Vec3<f32>,
  pub max: Vec3<f32>,
}

impl Box3 {
  pub fn new(min: Vec3<f32>, max: Vec3<f32>) -> Self {
    Box3 { min, max }
  }
  pub fn empty() -> Self {
    Box3::new(
      Vec3::new(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY),
      Vec3::new(
        std::f32::NEG_INFINITY,
        std::f32::NEG_INFINITY,
        std::f32::NEG_INFINITY,
      ),
    )
  }

  pub fn expand_by_point(&mut self, point: Vec3<f32>) {
    self.min.min(point);
    self.max.max(point);
  }

  pub fn make_from_position_buffer(position: &[f32]) -> Self {
    let mut b = Box3::empty();
    for index in 0..position.len() / 3 {
      let i = index * 3;
      b.expand_by_point(Vec3::new(position[i], position[i + 1], position[i + 2]));
    }
    b
  }

  pub fn apply_matrix(&mut self, mat: &Mat4<f32>) -> Self{
    unimplemented!()
  }
}