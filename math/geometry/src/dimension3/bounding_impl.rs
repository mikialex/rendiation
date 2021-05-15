use crate::{Box3, LineSegment, Point, SpaceBounding, Sphere, Triangle};
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

impl<T> SpaceBounding<T, Sphere<T>, 3> for Box3<T>
where
  T: Scalar,
{
  #[inline(always)]
  fn to_bounding(&self) -> Sphere<T> {
    let center = (self.max + self.min) * T::half();
    let radius = (self.max - center).length();
    Sphere::new(center, radius)
  }
}
