use crate::*;

impl<T, U> SpaceBounding<T, Box3<T>, 3> for Triangle<U>
where
  T: Scalar,
  U: Positioned<Position = Vec3<T>> + Copy,
{
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.map(|v| *v.position()).iter_point().collect()
  }
}

impl<T, U> SpaceBounding<T, Box3<T>, 3> for LineSegment<U>
where
  T: Scalar,
  U: Positioned<Position = Vec3<T>> + Copy,
{
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    self.map(|v| *v.position()).iter_point().collect()
  }
}

impl<T, U> SpaceBounding<T, Box3<T>, 3> for Point<U>
where
  T: Scalar,
  U: Positioned<Position = Vec3<T>> + Copy,
{
  #[inline(always)]
  fn to_bounding(&self) -> Box3<T> {
    [self.map(|v| *v.position()).0].iter().collect()
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
