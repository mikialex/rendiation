use crate::{Box3, HyperSphere, LebesgueMeasurable};
use rendiation_algebra::*;

pub type Sphere<T = f32> = HyperSphere<T, Vec3<T>>;

impl<T: Scalar> LebesgueMeasurable<T, 3> for Sphere<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::eval::<{ 3.0 / 4.0 * std::f32::consts::PI }>() * self.radius * self.radius * self.radius
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 2> for Sphere<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::eval::<{ 4.0 * std::f32::consts::PI }>() * self.radius * self.radius
  }
}

impl<T: Scalar + 'static> Sphere<T> {
  // we cant impl from iter trait because it need iter twice
  pub fn from_points<'a, I>(items: &'a I) -> Self
  where
    &'a I: IntoIterator<Item = &'a Vec3<T>>,
  {
    let box3: Box3<T> = items.into_iter().collect();
    let center = (box3.max + box3.min) * T::half();
    let mut max_distance2 = T::zero();
    items.into_iter().for_each(|&point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }

  pub fn from_points_and_center<'a, I>(items: &'a I, center: Vec3<T>) -> Self
  where
    &'a I: IntoIterator<Item = &'a Vec3<T>>,
  {
    let mut max_distance2 = T::zero();
    items.into_iter().for_each(|&point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }
}
