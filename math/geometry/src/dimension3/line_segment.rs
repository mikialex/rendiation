use crate::LineSegment;
use rendiation_algebra::{InnerProductSpace, Mat4, Scalar, Vec3};
use std::ops::Mul;

pub type LineSegment3D<T = f32> = LineSegment<Vec3<T>>;

impl<T: Scalar> LineSegment3D<T> {
  pub fn length(&self) -> T {
    self.start.distance(self.end)
  }
}

impl Mul<Mat4<f32>> for LineSegment3D {
  type Output = Self;

  fn mul(self, m: Mat4<f32>) -> Self {
    Self::new(self.start * m, self.end * m)
  }
}
