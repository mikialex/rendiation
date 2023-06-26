use rendiation_algebra::*;
use rendiation_color::*;
use rendiation_geometry::*;

mod path;
mod shape;
mod style;

pub use path::*;
pub use shape::*;
pub use style::*;

pub trait PainterAPI {
  fn reset(&mut self);

  type Image;
  fn register_image(&mut self, image: &Self::Image) -> TextureHandle;
  fn release_image(&mut self, image: &TextureHandle);

  /// baked data is the lossless snapshot of a painter API's drawing result. lossless means it's
  /// preserves vector representation, not rasterized image
  type Baked;
  fn draw_bake(&mut self, p: &Self::Baked);
  fn bake(self) -> Self::Baked;
  fn render_baked(&mut self, baked: &Self::Baked) -> Self::Image;

  fn stock_shape(&mut self, shape: &Shape, fill: &StrokeStyle);
  fn fill_shape(&mut self, shape: &Shape, fill: &FillStyle);

  fn push_transform(&mut self, transform: Mat3<f32>);
  fn pop_transform(&self) -> Mat3<f32>;

  fn push_mask(&mut self, mask: &Self::Baked);
  fn pop_mask(&mut self) -> Self::Baked;

  fn push_filter(&mut self);
  fn pop_filter(&mut self);
}

pub type TextureHandle = usize;
