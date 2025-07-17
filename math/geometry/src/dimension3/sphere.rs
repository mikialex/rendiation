use crate::*;

pub type Sphere<T = f32> = HyperSphere<T, Vec3<T>>;

impl<T: Scalar> LebesgueMeasurable<T, 3> for Sphere<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::eval::<{ scalar_transmute(3.0 / 4.0 * std::f32::consts::PI) }>()
      * self.radius
      * self.radius
      * self.radius
  }
}

impl<T: Scalar> LebesgueMeasurable<T, 2> for Sphere<T> {
  #[inline(always)]
  fn measure(&self) -> T {
    T::eval::<{ scalar_transmute(4.0 * std::f32::consts::PI) }>() * self.radius * self.radius
  }
}

impl<T: Scalar> Sphere<T> {
  // we cant impl from iter trait because it need iter twice
  pub fn from_points<I>(items: I) -> Self
  where
    I: IntoIterator<Item = Vec3<T>> + Clone,
  {
    let box3: Box3<T> = items.clone().into_iter().collect();
    let center = (box3.max + box3.min) * T::half();
    let mut max_distance2 = T::zero();
    items.into_iter().for_each(|point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }

  // we cant impl from iter trait because it need iter twice
  pub fn from_spheres<I>(items: I) -> Self
  where
    I: IntoIterator<Item = Self> + Clone,
  {
    let (center, weight) =
      items
        .clone()
        .into_iter()
        .fold((Vec3::zero(), T::zero()), |(center, weight), sphere| {
          (
            center + sphere.center * sphere.radius,
            weight + sphere.radius,
          )
        });
    let center = center / weight;

    let radius = items.into_iter().fold(T::zero(), |radius, sphere| {
      radius.max((sphere.center - center).length() + sphere.radius)
    });

    Self::new(center, radius)
  }

  pub fn from_points_and_center<I>(items: I, center: Vec3<T>) -> Self
  where
    I: IntoIterator<Item = Vec3<T>>,
  {
    let mut max_distance2 = T::zero();
    items.into_iter().for_each(|point| {
      let d = (point - center).length2();
      max_distance2 = max_distance2.max(d);
    });
    Sphere::new(center, max_distance2.sqrt())
  }
}
