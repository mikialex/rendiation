use crate::{Box3, LineSegment, Point, SpaceBounding, Triangle};
use rendiation_algebra::*;
use std::ops::{Deref, DerefMut};

impl<T, U> SpaceBounding<T, Box3<T>, 3> for Triangle<U>
where
  T: Scalar,
  U: Deref<Target = Vec3<T>> + DerefMut + Copy,
{
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.map(|v| *v).iter_point().collect()
  }
}

impl<T, U> SpaceBounding<T, Box3<T>, 3> for LineSegment<U>
where
  T: Scalar,
  U: Deref<Target = Vec3<T>> + DerefMut + Copy,
{
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.map(|v| *v).iter_point().collect()
  }
}

impl<T, U> SpaceBounding<T, Box3<T>, 3> for Point<U>
where
  T: Scalar,
  U: Deref<Target = Vec3<T>> + DerefMut + Copy,
{
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    [self.map(|v| *v).0].iter().collect()
  }
}
