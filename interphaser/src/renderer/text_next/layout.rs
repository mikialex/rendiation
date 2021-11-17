use crate::TextInfo;

use super::{GlyphID, GlyphRasterInfo};

pub struct LayoutedTextGlyphs {
  glyphs: Vec<(GlyphID, GlyphRasterInfo)>,
}

pub trait TextGlyphLayouter {
  fn layout(text: &TextInfo) -> LayoutedTextGlyphs;
}

pub struct GlyphBrushLayouter {}

impl TextGlyphLayouter for GlyphBrushLayouter {
    fn layout(text: &TextInfo) -> LayoutedTextGlyphs {
        todo!()
    }
}
