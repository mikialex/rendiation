use rendiation_color::*;
pub type Color = ColorWithAlpha<SRGBColor<f32>, f32>;

pub struct SolidFillStyle {
  pub color: Color,
  pub alpha: f32,
}

pub struct TextureId;

pub enum FillStyle {
  Solid(SolidFillStyle),
  Texture(TextureId),
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
