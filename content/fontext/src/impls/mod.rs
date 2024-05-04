use glyph_brush::ab_glyph::point;
use glyph_brush::ab_glyph::Font;
use glyph_brush::*;
use glyph_brush::{ab_glyph, FontId};
use rendiation_texture_core::Texture2dInitAble;
use rendiation_texture_packer::pack_2d_to_2d::pack_impl::etagere_wrap::EtagerePacker;

use crate::*;

impl TextCache {
  pub fn new_default_impl(init_size: Size) -> Self {
    let tolerance = Default::default();

    let packer = EtagerePacker::default();

    let glyph_cache = GlyphCache::new(init_size, tolerance, packer);

    Self::new(glyph_cache, GlyphBrushLayouter)
  }
}

impl FontManager {
  pub fn new_with_default_font() -> Self {
    let mut fonts = Self::default();

    #[cfg(not(target_arch = "wasm32"))]
    {
      let property = font_loader::system_fonts::FontPropertyBuilder::new()
        .family("Arial")
        .build();

      let (font, _) = font_loader::system_fonts::get(&property).unwrap();
      let default_font = ab_glyph::FontArc::try_from_vec(font).unwrap();
      fonts.add_font("default", default_font);
    }

    #[cfg(target_arch = "wasm32")]
    {
      let default_font = include_bytes!("./CascadiaMonoPL-Regular.otf");
      let default_font = ab_glyph::FontArc::try_from_slice(default_font).unwrap();
      fonts.add_font("default", default_font);
    }

    fonts
  }
}

impl crate::Font for ab_glyph::FontArc {
  fn raster(&self, glyph_id: GlyphId, info: GlyphRasterInfo) -> Option<Texture2DBuffer<u8>> {
    let glyph_id = ab_glyph::GlyphId(glyph_id.0 as u16);
    let font = self;
    fn into_unsigned_u8(f: f32) -> u8 {
      (f * 255.) as u8
    }

    let q_glyph =
      glyph_id.with_scale_and_position(info.scale, point(info.position.x, info.position.y));

    // Draw it.
    let outlined_glyph = font.outline_glyph(q_glyph)?;
    let bounds = outlined_glyph.px_bounds();
    let width = bounds.width().ceil() as usize;
    let height = bounds.height().ceil() as usize;
    let size = Size::from_usize_pair_min_one((width, height));

    let mut result = Texture2DBuffer::init_not_care(size);
    outlined_glyph.draw(|x, y, c| result.write((x as usize, y as usize), into_unsigned_u8(c)));

    result.into()
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[derive(Default)]
pub struct GlyphBrushLayouter;

fn convert_align_h(v: crate::TextHorizontalAlignment) -> glyph_brush::HorizontalAlign {
  match v {
    TextHorizontalAlignment::Left => glyph_brush::HorizontalAlign::Left,
    TextHorizontalAlignment::Center => glyph_brush::HorizontalAlign::Center,
    TextHorizontalAlignment::Right => glyph_brush::HorizontalAlign::Right,
  }
}

fn convert_align_v(v: crate::TextVerticalAlignment) -> glyph_brush::VerticalAlign {
  match v {
    crate::TextVerticalAlignment::Center => glyph_brush::VerticalAlign::Center,
    crate::TextVerticalAlignment::Top => glyph_brush::VerticalAlign::Top,
    crate::TextVerticalAlignment::Bottom => glyph_brush::VerticalAlign::Bottom,
  }
}

impl TextGlyphLayouter for GlyphBrushLayouter {
  fn layout(&self, text: &TextInfo, fonts: &FontManager) -> LayoutedTextGlyphs {
    let x_correct = match text.horizon_align {
      crate::TextHorizontalAlignment::Left => 0.,
      crate::TextHorizontalAlignment::Center => text.bounds.0 / 2.,
      crate::TextHorizontalAlignment::Right => text.bounds.0,
    };

    let y_correct = match text.vertical_align {
      crate::TextVerticalAlignment::Top => 0.,
      crate::TextVerticalAlignment::Center => text.bounds.1 / 2.,
      crate::TextVerticalAlignment::Bottom => text.bounds.1,
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
      bounds: text.bounds,
    };

    // this is costly, but hard to workaround
    let font_list: Vec<_> = fonts
      .get_fonts()
      .iter()
      .map(|f| f.as_any().downcast_ref::<ab_glyph::FontArc>().unwrap())
      .collect();

    let raw_result = layout.calculate_glyphs(
      font_list.as_slice(),
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
        let font_id = crate::FontId(r.font_id.0);
        let font = fonts
          .get_font(font_id)?
          .as_any()
          .downcast_ref::<ab_glyph::FontArc>()
          .unwrap();

        let outlined_glyph = font.outline_glyph(r.glyph.clone())?;
        let bounds = outlined_glyph.px_bounds();

        let rect = rendiation_geometry::Rectangle {
          min: (bounds.min.x, bounds.min.y).into(),
          max: (bounds.max.x, bounds.max.y).into(),
        };
        bound.get_or_insert(rect).union(rect);

        let glyph_id = font.glyph_id(c);
        let glyph_id = GlyphId(glyph_id.0 as u32);

        (
          FontGlyphId { font_id, glyph_id },
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

    LayoutedTextGlyphs {
      source: text.content.clone(),
      glyphs,
      bound,
    }
  }
}
