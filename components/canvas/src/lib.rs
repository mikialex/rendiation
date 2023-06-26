use rendiation_algebra::*;
use rendiation_color::*;
use rendiation_geometry::*;

mod effect;
mod impls;
mod path;
mod shape;
mod style;

pub use effect::*;
pub use impls::*;
pub use path::*;
pub use shape::*;
pub use style::*;

pub trait PainterAPI {
  fn reset(&mut self);

  type Image;
  fn register_image(&mut self, image: Self::Image) -> TextureHandle;
  fn render(&self) -> Self::Image;

  /// baked data is the lossless snapshot of a painter API's drawing result. lossless means it's
  /// preserves vector representation, but not rasterized image, and keep better (but not necessary
  /// perfect) quality when apply transformation by parent ctx.
  ///
  /// Another design purpose of this type is to group the painter representation in order to
  /// cache the compute. The drawing cost of the baked object should be cheaper than the
  /// collection of discrete drawing commands. This provides a transparent way to express the
  /// system's caching capability
  type Baked;
  fn draw_bake(&mut self, p: &Self::Baked);
  fn bake(self) -> Self::Baked;

  fn draw_baked(&mut self, baked: Self::Baked);
  fn stroke_shape(&mut self, shape: &Shape, fill: &StrokeStyle);
  fn fill_shape(&mut self, shape: &Shape, fill: &FillStyle);

  fn push_transform(&mut self, transform: Mat3<f32>);
  fn pop_transform(&self) -> Mat3<f32>;

  fn push_mask(&mut self, mask: Self::Baked);
  fn pop_mask(&mut self) -> Option<Self::Baked>;

  fn push_filter(&mut self, effect: CanvasEffect);
  fn pop_filter(&mut self) -> CanvasEffect;
}

pub type TextureHandle = usize;
