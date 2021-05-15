use crate::LineSegment;
use rendiation_algebra::{InnerProductSpace, Scalar, Vec3};

pub type LineSegment3D<T = f32> = LineSegment<Vec3<T>>;

impl<T: Scalar> LineSegment3D<T> {
  pub fn length(&self) -> T {
    self.start.distance(self.end)
  }
}
