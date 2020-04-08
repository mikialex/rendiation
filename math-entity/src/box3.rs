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
    const INF: f32 = std::f32::INFINITY;
    const N_INF: f32 = std::f32::NEG_INFINITY;
    Box3::new(Vec3::new(INF, INF, INF), Vec3::new(N_INF, N_INF, N_INF))
  }

  pub fn center(&self) -> Vec3<f32> {
    (self.min + self.max) * 0.5
  }

  pub fn expand_by_point(&mut self, point: Vec3<f32>) {
    self.min.min(point);
    self.max.max(point);
  }

  pub fn new_from_position_data<'a, T>(iter: &mut T) -> Self
  where
    T: Iterator<Item = &'a Vec3<f32>>,
  {
    let mut b = Box3::empty();
    iter.for_each(|p| b.expand_by_point(*p));
    b
  }
}

use std::ops::Mul;
impl Mul<Mat4<f32>> for Box3 {
  type Output = Self;

  fn mul(self, m: Mat4<f32>) -> Self {
    Self::new(self.min * m, self.max * m)
  }
}
