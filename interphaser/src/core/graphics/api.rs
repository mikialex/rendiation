use crate::{FillStyle, Path2dBuilder, Shape, StrokeStyle};
use arena::*;

pub trait PainterAPI {
  // type Image;

  // fn register_image(&mut self, image: &Self::Image) -> TextureHandle;
  // fn deregister_image(&mut self, image: &TextureHandle);

  fn stock_shape(&mut self, shape: &impl Shape, fill: &StrokeStyle);
  fn fill_shape(&mut self, shape: &impl Shape, fill: &FillStyle);
}

#[derive(Clone)]
pub struct TextureHandle {
  // pool: ImagePool<T>,
}

pub struct ImagePool<T> {
  images: Arena<T>,
}

pub struct Painter {
  path_builder: Path2dBuilder,
  // images: ImagePool
}
