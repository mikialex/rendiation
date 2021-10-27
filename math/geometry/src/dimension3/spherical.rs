use rendiation_algebra::*;

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

  pub fn to_vec3(&self) -> Vec3<T> {
    let sin_radius = self.polar.sin() * self.radius;
    Vec3::new(
      sin_radius * self.azim.sin(),
      self.radius * self.polar.cos(),
      sin_radius * self.azim.cos(),
    ) + self.center
  }

  pub fn from_vec3_and_center(forward: Vec3<T>, eye: Vec3<T>) -> Self {
    let dir = forward.reverse();

    let radius = dir.length();
    let polar = (dir.y / radius).acos();
    let azim = (dir.x / polar.sin() * radius).asin();

    Self {
      radius,
      polar,
      azim,
      center: eye + forward,
    }
  }
}
