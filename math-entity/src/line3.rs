use rendiation_math::Mat4;
use rendiation_math::Vec3;

#[derive(Clone)]
pub struct Line3 {
  pub start: Vec3<f32>,
  pub end: Vec3<f32>,
}

impl Line3 {
  pub fn new(start: Vec3<f32>, end: Vec3<f32>) -> Self {
    Self { start, end }
  }

  pub fn length(&self) -> f32 {
    (self.start - self.end).length()
  }
}

use std::ops::Mul;
impl Mul<Mat4<f32>> for Line3 {
  type Output = Self;

  fn mul(self, m: Mat4<f32>) -> Self {
    Self::new(self.start * m, self.end * m)
  }
}
