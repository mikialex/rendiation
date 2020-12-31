use rendiation_math::Scalar;

use crate::{Box3, LineSegment, Point, Positioned, SpaceBounding, Triangle};

impl<T: Scalar, V: Positioned<T, 3>> SpaceBounding<T, Box3<T>, 3> for Triangle<V> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.iter_point().map(|v| v.position()).collect()
  }
}

impl<T: Scalar, V: Positioned<T, 3>> SpaceBounding<T, Box3<T>, 3> for LineSegment<V> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.iter_point().map(|v| v.position()).collect()
  }
}

impl<T: Scalar, V: Positioned<T, 3>> SpaceBounding<T, Box3<T>, 3> for Point<V> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    [self.0].iter().map(|v| v.position()).collect()
  }
}
