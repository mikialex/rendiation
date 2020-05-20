use crate::{Index, SceneGraphBackEnd};
use std::ops::Range;

pub trait Geometry<T: SceneGraphBackEnd> {
  fn provide_gpu(&mut self, renderer: &mut T::Renderer);
  fn get_draw_range(&self) -> Range<u32>;
}

pub struct SceneGeometry<T: SceneGraphBackEnd> {
  index: Index,
  data: Box<dyn Geometry<T>>,
}
