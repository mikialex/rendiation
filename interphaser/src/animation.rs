pub trait AnimateAble {
  fn interpolate(&self, target: &Self, time: f32) -> Self;
}
