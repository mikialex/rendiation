use crate::TextInfo;

use super::{GlyphID, GlyphRasterInfo};

pub struct LayoutedTextGlyphs {
  pub glyphs: Vec<(GlyphID, GlyphRasterInfo)>,
}

pub trait TextGlyphLayouter {
  fn layout(&self, text: &TextInfo) -> LayoutedTextGlyphs;
}

#[derive(Default)]
pub struct GlyphBrushLayouter {}

impl TextGlyphLayouter for GlyphBrushLayouter {
  fn layout(&self, text: &TextInfo) -> LayoutedTextGlyphs {
    todo!()
  }
}
