use crate::TextInfo;

pub struct LayoutedTextGlyphs {
  glyphs: Vec<usize>,
}

pub trait TextGlyphLayouter {
  fn layout(text: &TextInfo) -> LayoutedTextGlyphs;
}

pub struct GlyphBrushLayouter {}
