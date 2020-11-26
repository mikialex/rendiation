use rendiation_math::Vec3;

use crate::{Box3, LineSegment3D, Point, SpaceBounding, Triangle};

impl SpaceBounding<Box3> for Triangle<f32, 3> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3 {
    self.iter_point().collect()
  }
}

impl SpaceBounding<Box3> for LineSegment3D {
  #[inline(always)]
  fn to_bounding(&self) -> Box3 {
    self.iter_point().collect()
  }
}

impl SpaceBounding<Box3> for Point<Vec3<f32>> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3 {
    [self.0].iter().collect()
  }
}
