pub trait ParametricSurface {
  fn sample(&self, position: Vec2<f32>) -> Vec3<f32>;
}
