use rendiation_math::*;

#[derive(Debug, Copy, Clone)]
pub struct Ray {
  pub origin: Vec3<f32>,
  pub direction: Vec3<f32>,
}

impl Ray {
  pub fn new(origin: Vec3<f32>, direction: Vec3<f32>) -> Self {
    Ray { origin, direction }
  }

  pub fn from_point_to_point(origin: Vec3<f32>, target: Vec3<f32>) -> Self {
    Ray::new(origin, (target - origin).normalize())
  }

  pub fn at(&self, distance: f32) -> Vec3<f32> {
    self.origin + self.direction * distance
  }
}
