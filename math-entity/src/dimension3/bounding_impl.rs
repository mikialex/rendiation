use rendiation_math::Vec3;

use crate::{Box3, LineSegment3D, Point, Triangle};

impl From<Triangle> for Box3 {
  #[inline(always)]
  fn from(v: Triangle) -> Self {
    v.iter_point().collect()
  }
}

impl From<LineSegment3D> for Box3 {
  #[inline(always)]
  fn from(v: LineSegment3D) -> Self {
    v.iter_point().collect()
  }
}

impl From<Point<Vec3<f32>>> for Box3 {
  #[inline(always)]
  fn from(v: Point<Vec3<f32>>) -> Self {
    [v.0].iter().collect()
  }
}
