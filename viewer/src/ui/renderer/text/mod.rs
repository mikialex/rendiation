mod cache;
use cache::*;
mod pipeline;
use pipeline::*;

pub struct TextRenderer {
  cache: Cache,
  brush: glyph_brush::GlyphBrush<Instance, Extra, F, H>,
}
