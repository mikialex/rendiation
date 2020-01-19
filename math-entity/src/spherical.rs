use rendiation_math::Vec3;

pub struct Spherical {
  pub center: Vec3<f32>,
  pub radius: f32,
  pub polar: f32,
  pub azim: f32,
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

  pub fn reset_pose(&mut self){
    self.radius = 0.;
    self.polar = 0.;
    self.azim = 0.;
  }
}
