use crate::{FillStyle, Path2dBuilder, StrokeStyle};

pub trait Canvas2DContextAPI {
  type Image;

  fn register_image(&mut self, image: &Self::Image) -> TextureHandle;
  fn deregister_image(&mut self, image: &TextureHandle);

  fn fill_style(&mut self, fill: &FillStyle);
  fn stock_style(&mut self, fill: &StrokeStyle);
  fn fill_shape(&mut self, shape: &impl Shape);
}

pub struct TextureHandle {
  //
}

pub trait Shape {
  fn create_path(&self, builder: &mut Path2dBuilder);
}
