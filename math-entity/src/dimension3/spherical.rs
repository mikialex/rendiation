use rendiation_math::Vec3;

pub struct Spherical<T = f32> {
  pub center: Vec3<T>,
  pub radius: T,
  pub polar: T,
  pub azim: T,
}

impl Spherical {
  pub fn new() -> Self {
    Spherical {
      center: Vec3::new(0.0, 0.0, 0.0),
      radius: 1.,
      polar: 0.,
      azim: 0.,
    }
  }

  pub fn reset_pose(&mut self) {
    self.radius = 1.;
    self.polar = 0.;
    self.azim = 0.;
  }

  pub fn to_vec3(&self) -> Vec3<f32> {
    let sin_radius = self.polar.sin() * self.radius;
    Vec3::new(
      sin_radius * self.azim.sin(),
      self.radius * self.polar.cos(),
      sin_radius * self.azim.cos(),
    ) + self.center
  }
}
