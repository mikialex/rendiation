use rendiation_math::Vec3;

pub struct Spherical {
  center: Vec3<f32>,
  radius: f32,
  polar: f32,
  azim: f32,
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
}
