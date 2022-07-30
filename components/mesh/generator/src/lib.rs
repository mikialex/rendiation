use rendiation_algebra::*;

pub trait ParametricSurface {
  fn sample(&self, position: Vec2<f32>) -> Vec3<f32>;
}

pub trait ParametricCurve {
  fn sample(&self, position: f32) -> Vec3<f32>;
}
