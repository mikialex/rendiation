use crate::{FillStyle, Path2dBuilder, StrokeStyle};

pub trait Canvas2DContextAPI {
  fn fill_style(&mut self, fill: &FillStyle);
  fn stock_style(&mut self, fill: &StrokeStyle);
  fn fill_shape(&mut self, shape: &dyn Shape);
}

pub trait Shape {
  fn create_path(&self, builder: &mut Path2dBuilder);
}
