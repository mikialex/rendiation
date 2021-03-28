use rendiation_algebra::*;

use crate::{Box3, LineSegment, Point, Positioned, SpaceBounding, Triangle};

impl<T: Scalar, V: Positioned<T, 3>> SpaceBounding<T, Box3<T>, 3> for Triangle<V> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.map_position().iter_point().collect()
  }
}

impl<T: Scalar, V: Positioned<T, 3>> SpaceBounding<T, Box3<T>, 3> for LineSegment<V> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.map_position().iter_point().collect()
  }
}

impl<T: Scalar, V: Positioned<T, 3>> SpaceBounding<T, Box3<T>, 3> for Point<V> {
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    [self.map_position().0].iter().collect()
  }
}
