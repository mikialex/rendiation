use crate::*;

#[derive(Clone, Copy, Debug, Facet)]
pub struct Spherical<T = f32> {
  pub center: Vec3<T>,
  pub radius: T,
  pub polar: T,
  pub azim: T,
}

impl<T: Scalar> Default for Spherical<T> {
  fn default() -> Self {
    let mut r = Self {
      center: Vec3::splat(T::zero()),
      radius: T::one(),
      polar: T::zero(),
      azim: T::zero(),
    };
    r.reset_pose();
    r
  }
}

impl<T: Scalar> Spherical<T> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn reset_pose(&mut self) {
    self.radius = T::one();
    self.polar = T::zero();
    self.azim = T::zero();
  }

  pub fn to_sphere_point(&self) -> Vec3<T> {
    let sin_radius = self.polar.sin() * self.radius;
    Vec3::new(
      sin_radius * self.azim.cos(),
      self.radius * self.polar.cos(),
      sin_radius * self.azim.sin(),
    ) + self.center
  }

  pub fn from_sphere_point_and_center(forward: Vec3<T>, eye: Vec3<T>) -> Self {
    let dir = forward.reverse();

    let radius = dir.length();
    let polar = (dir.y / radius).max(-T::one()).min(T::one()).acos();
    let mut azim = (dir.x / (polar.sin() * radius))
      .max(T::zero())
      .min(T::one())
      .acos();

    if dir.z < T::zero() {
      azim = -azim;
    }

    Self {
      radius,
      polar,
      azim,
      center: eye + forward,
    }
  }
}
