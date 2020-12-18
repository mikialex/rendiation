use crate::LineSegment;
use rendiation_math::{InnerProductSpace, Mat4, Vec3};
use std::ops::Mul;

pub type LineSegment3D = LineSegment<Vec3<f32>>;

impl LineSegment3D {
  pub fn length(&self) -> f32 {
    self.start.distance(self.end)
  }
}

impl Mul<Mat4<f32>> for LineSegment3D {
  type Output = Self;

  fn mul(self, m: Mat4<f32>) -> Self {
    Self::new(self.start * m, self.end * m)
  }
}
