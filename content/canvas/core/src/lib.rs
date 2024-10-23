use rendiation_algebra::*;
use rendiation_color::*;

mod effect;
mod path;
mod shape;
mod style;

pub use effect::*;
pub use path::*;
pub use shape::*;
pub use style::*;

pub trait PainterAPI {
  fn reset(&mut self);

  type Image;
  fn register_image(&mut self, image: Self::Image) -> TextureHandle;
  fn render(&mut self, target: &Self::Image);

  /// baked data is the lossless snapshot of a painter API's drawing result. lossless means it's
  /// preserves vector representation, but not rasterized image, and keep better (but not necessary
  /// perfect) quality when apply transformation by parent ctx.
  ///
  /// Another design purpose of this type is to group the painter representation in order to
  /// cache the compute. The drawing cost of the baked object should be cheaper than the
  /// query of discrete drawing commands. This provides a transparent way to express the
  /// system's caching capability
  type Baked;
  fn draw_bake(&mut self, p: &Self::Baked);
  fn bake(self) -> Self::Baked;

  fn stroke_shape(&mut self, shape: &Shape, style: &StrokeStyle);
  fn fill_shape(&mut self, shape: &Shape, style: &FillStyle);

  fn push_transform(&mut self, transform: Mat3<f32>);
  fn pop_transform(&mut self) -> Option<Mat3<f32>>;

  fn push_mask(&mut self, mask: Self::Baked);
  fn pop_mask(&mut self) -> Option<Self::Baked>;

  fn push_filter(&mut self, effect: CanvasEffect);
  fn pop_filter(&mut self) -> Option<CanvasEffect>;
}

pub type TextureHandle = usize;

/// PainterAPI naturally support text by converting text to shape or bitmap by user.
/// However, it's only suitable for art text. For large paragraph of UI text,  the
/// painter maybe support custom way to draw them efficiently. Compare to the pervious way
/// the TextWriterExtensionAPI extend the ability to painter api but with more constraint text input
/// interface
pub trait TextWriterExtensionAPI: PainterAPI {
  fn write_text(&mut self, layouted_text: u32);
}
