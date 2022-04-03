use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

use rendiation_color::*;
use rendiation_texture::Size;
use rendiation_texture_packer::etagere_wrap::EtagerePacker;

pub mod cache_glyph;
pub use cache_glyph::*;

pub mod cache_text;
pub use cache_text::*;

pub mod layout;
pub use layout::*;

pub mod raster;
pub use raster::*;

pub mod packer;
pub use packer::*;

use crate::{HorizontalAlignment, VerticalAlignment};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LineWrap {
  Single,
  Multiple,
}

impl Default for LineWrap {
  fn default() -> Self {
    Self::Single
  }
}

type Color = ColorWithAlpha<SRGBColor<f32>, f32>;

#[derive(Debug, Clone)]
pub struct TextInfo {
  pub content: String,
  pub bounds: (f32, f32),
  pub line_wrap: LineWrap,
  pub horizon_align: HorizontalAlignment,
  pub vertical_align: VerticalAlignment,
  pub color: Color,
  pub font_size: f32,
  pub x: f32,
  pub y: f32,
}

pub struct TextRelaxedInfo {
  pub content: String,
  pub font_size: f32,
}

pub type TextHash = u64;

impl TextInfo {
  pub fn hash(&self) -> TextHash {
    let mut hasher = DefaultHasher::default();
    self.content.hash(&mut hasher);
    self.bounds.0.to_bits().hash(&mut hasher);
    self.bounds.1.to_bits().hash(&mut hasher);
    self.line_wrap.hash(&mut hasher);
    self.horizon_align.hash(&mut hasher);
    self.vertical_align.hash(&mut hasher);
    self.color.r.to_bits().hash(&mut hasher);
    self.color.g.to_bits().hash(&mut hasher);
    self.color.b.to_bits().hash(&mut hasher);
    self.font_size.to_bits().hash(&mut hasher);
    self.x.to_bits().hash(&mut hasher);
    self.y.to_bits().hash(&mut hasher);
    hasher.finish()
  }
}

impl TextCache {
  pub fn new_default_impl(init_size: Size) -> Self {
    let tolerance = Default::default();

    let raster = AbGlyphRaster::default();

    let packer = EtagerePacker::default();

    let glyph_cache = GlyphCache::new(init_size, tolerance, raster, packer);

    Self::new(glyph_cache, GlyphBrushLayouter::default())
  }
}
