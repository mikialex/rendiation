use std::any::Any;
use rendiation_math::Mat4;

pub trait TransformedObject: Any {
  fn matrix(&self) -> &Mat4<f32>;
  fn matrix_mut(&mut self) -> &mut Mat4<f32>;

  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
