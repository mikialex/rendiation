use crate::*;

pub struct LayoutedTextGlyphs {
  pub source: String,
  pub glyphs: Vec<(FontGlyphId, GlyphRasterInfo, GlyphBound)>,
  pub bound: Option<Rectangle>,
}

pub trait TextGlyphLayouter {
  fn layout(&self, text: &TextInfo, fonts: &FontManager) -> LayoutedTextGlyphs;
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct TextQuadInstance {
  bound: GlyphBound,
  tex_left_top: [f32; 2],
  tex_right_bottom: [f32; 2],
  color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct GlyphBound {
  pub left_top: [f32; 3],
  pub right_bottom: [f32; 2],
}

impl LayoutedTextGlyphs {
  pub fn generate_gpu_vertex(&self, cache: &GlyphCache) -> Vec<TextQuadInstance> {
    self
      .glyphs
      .iter()
      .filter_map(|(gid, info, bound)| {
        let (tex_left_top, tex_right_bottom) = cache.get_cached_glyph_info(*gid, *info)?;

        TextQuadInstance {
          bound: *bound,
          tex_left_top,
          tex_right_bottom,
          color: [0., 0., 0., 1.],
        }
        .into()
      })
      .collect()
  }
}
