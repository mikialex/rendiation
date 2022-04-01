use glyph_brush::ab_glyph::Font;
use glyph_brush::*;

use crate::{FontManager, HorizontalAlignment, Rectangle};

use super::{GlyphCache, GlyphID, GlyphRasterInfo, TextInfo};

pub struct LayoutedTextGlyphs {
  pub glyphs: Vec<(GlyphID, GlyphRasterInfo, GlyphBound)>,
  pub bound: Option<Rectangle>,
}

pub trait TextGlyphLayouter {
  fn layout(&self, text: &TextInfo, fonts: &FontManager) -> LayoutedTextGlyphs;
}

#[derive(Default)]
pub struct GlyphBrushLayouter;

fn convert_align_h(v: crate::HorizontalAlignment) -> glyph_brush::HorizontalAlign {
  match v {
    HorizontalAlignment::Left => glyph_brush::HorizontalAlign::Left,
    HorizontalAlignment::Center => glyph_brush::HorizontalAlign::Center,
    HorizontalAlignment::Right => glyph_brush::HorizontalAlign::Right,
  }
}

fn convert_align_v(v: crate::VerticalAlignment) -> glyph_brush::VerticalAlign {
  match v {
    crate::VerticalAlignment::Center => glyph_brush::VerticalAlign::Center,
    crate::VerticalAlignment::Top => glyph_brush::VerticalAlign::Top,
    crate::VerticalAlignment::Bottom => glyph_brush::VerticalAlign::Bottom,
  }
}

impl TextGlyphLayouter for GlyphBrushLayouter {
  fn layout(&self, text: &TextInfo, fonts: &FontManager) -> LayoutedTextGlyphs {
    let x_correct = match text.horizon_align {
      crate::HorizontalAlignment::Left => 0.,
      crate::HorizontalAlignment::Center => text.bounds.width / 2.,
      crate::HorizontalAlignment::Right => text.bounds.width,
    };

    let y_correct = match text.vertical_align {
      crate::VerticalAlignment::Top => 0.,
      crate::VerticalAlignment::Center => text.bounds.height / 2.,
      crate::VerticalAlignment::Bottom => text.bounds.height,
    };

    let layout = match text.line_wrap {
      crate::LineWrap::Single => Layout::SingleLine {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: convert_align_h(text.horizon_align),
        v_align: convert_align_v(text.vertical_align),
      },
      crate::LineWrap::Multiple => Layout::Wrap {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: convert_align_h(text.horizon_align),
        v_align: convert_align_v(text.vertical_align),
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

    let mut bound = None;

    let glyphs = raw_result
      .iter()
      .zip(text.content.chars().filter(|c| !c.is_control()))
      .filter_map(|(r, c)| {
        let font = fonts.get_font(r.font_id);

        let outlined_glyph = font.outline_glyph(r.glyph.clone())?;
        let bounds = outlined_glyph.px_bounds();

        let rect = Rectangle {
          min: (bounds.min.x, bounds.min.y).into(),
          max: (bounds.max.x, bounds.max.y).into(),
        };
        bound.get_or_insert(rect).union(rect);

        (
          GlyphID(c, r.font_id),
          GlyphRasterInfo {
            position: (r.glyph.position.x, r.glyph.position.y).into(),
            scale: r.glyph.scale.x,
          },
          GlyphBound {
            left_top: [bounds.min.x, bounds.min.y, 0.],
            right_bottom: [bounds.max.x, bounds.max.y],
          },
        )
          .into()
      })
      .collect();

    LayoutedTextGlyphs { glyphs, bound }
  }
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