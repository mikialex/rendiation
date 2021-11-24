use glyph_brush::*;

use crate::FontManager;

use super::{GlyphCache, GlyphID, GlyphRasterInfo, TextInfo};

pub struct LayoutedTextGlyphs {
  pub glyphs: Vec<(GlyphID, GlyphRasterInfo)>,
}

pub trait TextGlyphLayouter {
  fn layout(&self, text: &TextInfo, fonts: &FontManager) -> LayoutedTextGlyphs;
}

#[derive(Default)]
pub struct GlyphBrushLayouter;

impl TextGlyphLayouter for GlyphBrushLayouter {
  fn layout(&self, text: &TextInfo, fonts: &FontManager) -> LayoutedTextGlyphs {
    let x_correct = match text.horizon_align {
      glyph_brush::HorizontalAlign::Left => 0.,
      glyph_brush::HorizontalAlign::Center => text.bounds.width / 2.,
      glyph_brush::HorizontalAlign::Right => text.bounds.width,
    };

    let y_correct = match text.vertical_align {
      glyph_brush::VerticalAlign::Top => 0.,
      glyph_brush::VerticalAlign::Center => text.bounds.height / 2.,
      glyph_brush::VerticalAlign::Bottom => text.bounds.height / 2.,
    };

    let layout = match text.line_wrap {
      crate::LineWrap::Single => Layout::SingleLine {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Center,
      },
      crate::LineWrap::Multiple => Layout::Wrap {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Center,
      },
    };

    let geometry = SectionGeometry {
      screen_position: (text.x + x_correct, text.y + y_correct),
      bounds: text.bounds.into(),
    };

    let raw_result = layout.calculate_glyphs(
      fonts.get_fonts().as_slice(),
      &geometry,
      &[SectionText {
        text: text.content.as_str(),
        scale: ab_glyph::PxScale::from(text.font_size),
        font_id: FontId(0),
      }],
    );
    LayoutedTextGlyphs {
      glyphs: raw_result
        .iter()
        .zip(text.content.chars()) // todo seems buggy
        .map(|(r, c)| {
          (
            GlyphID(c, r.font_id),
            GlyphRasterInfo {
              position: (r.glyph.position.x, r.glyph.position.y).into(),
              scale: r.glyph.scale.x,
            },
          )
        })
        .collect(),
    }
  }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct TextQuadInstance {
  left_top: [f32; 3],
  right_bottom: [f32; 2],
  tex_left_top: [f32; 2],
  tex_right_bottom: [f32; 2],
  color: [f32; 4],
}

impl LayoutedTextGlyphs {
  pub fn generate_gpu_vertex(&self, cache: &GlyphCache) -> Vec<TextQuadInstance> {
    self
      .glyphs
      .iter()
      .filter_map(|(gid, info)| {
        let (tex_left_top, tex_right_bottom, (width, height)) =
          cache.get_cached_glyph_info(*gid, *info)?;

        TextQuadInstance {
          left_top: [info.position.x, info.position.y, 0.],
          right_bottom: [info.position.x + width, info.position.y + height],
          tex_left_top,
          tex_right_bottom,
          color: [0., 0., 0., 1.],
        }
        .into()
      })
      .collect()
  }
}
