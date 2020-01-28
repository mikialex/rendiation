use rendiation_math::*;

#[derive(Debug, Copy, Clone)]
pub struct Ray {
  origin: Vec3<f32>,
  direction: Vec3<f32>,
}

impl Ray {
  pub fn new(origin: Vec3<f32>, direction: Vec3<f32>) -> Self {
    Ray { origin, direction }
  }
}
