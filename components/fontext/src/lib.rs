use rendiation_geometry::Rectangle;

use rendiation_texture::{Size, Texture2D, Texture2DBuffer, TextureRange};
use rendiation_texture_packer::etagere_wrap::EtagerePacker;
use rendiation_texture_packer::{PackError, PackId, PackerConfig, RePackablePacker};

use glyph_brush::ab_glyph::{point, Font};
use glyph_brush::*;
use glyph_brush::{ab_glyph, FontId};
use linked_hash_map::LinkedHashMap;

use rendiation_algebra::Vec2;
use rendiation_color::*;

use std::{
  cell::RefCell,
  collections::hash_map::DefaultHasher,
  collections::{HashMap, HashSet},
  hash::{Hash, Hasher},
  rc::Rc,
};

pub mod text;
pub use text::*;

pub mod fonts;
pub use fonts::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TextHorizontalAlignment {
  Center,
  Left,
  Right,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TextVerticalAlignment {
  Center,
  Top,
  Bottom,
}

impl Default for TextHorizontalAlignment {
  fn default() -> Self {
    Self::Center
  }
}

impl Default for TextVerticalAlignment {
  fn default() -> Self {
    Self::Center
  }
}
