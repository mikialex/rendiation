use crate::*;

pub type DisplayColor = ColorWithAlpha<SRGBColor<f32>, f32>;

pub struct SolidFillStyle {
  pub color: DisplayColor,
  pub alpha: f32,
}

pub enum FillStyle {
  Solid(SolidFillStyle),
  Texture(TextureHandle),
}

pub struct StrokeStyle {
  pub width: f32,
  pub line_cap: LineCap,
  pub line_join: LineJoin,
}

pub enum LineCap {
  Butt,
  Square,
  Round,
}

pub enum LineJoin {
  Miter,
  Round,
  Bevel,
}
