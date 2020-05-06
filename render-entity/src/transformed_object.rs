use rendiation_math_entity::Transformation;
use std::any::Any;

pub trait TransformedObject: Any {
  fn get_transform(&self) -> &Transformation;
  fn get_transform_mut(&mut self) -> &mut Transformation;

  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
