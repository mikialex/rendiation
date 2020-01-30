use crate::ray::Ray;
use rendiation_math::vec::Math;
use rendiation_math::*;

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

  pub fn center(&self) -> Vec3<f32> {
    (self.min + self.max) * 0.5
  }

  pub fn expand_by_point(&mut self, point: Vec3<f32>) {
    self.min.min(point);
    self.max.max(point);
  }

  pub fn new_from_position_data<'a, T: Iterator<Item = &'a Vec3<f32>>>(iter: T) -> Self {
    let mut b = Box3::empty();
    for point in iter {
      b.expand_by_point(*point);
    }
    b
  }

  pub fn apply_matrix(&mut self, mat: &Mat4<f32>) -> Self {
    todo!()
  }

  pub fn if_ray_hit(&self, ray: &Ray) -> bool {
    todo!()
  }
}
