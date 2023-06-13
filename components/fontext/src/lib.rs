use std::{
  any::Any,
  cell::RefCell,
  collections::hash_map::DefaultHasher,
  collections::{HashMap, HashSet},
  hash::{Hash, Hasher},
  rc::Rc,
};

use linked_hash_map::LinkedHashMap;
use rendiation_algebra::Vec2;
use rendiation_color::*;
use rendiation_geometry::Rectangle;
use rendiation_texture::{Size, Texture2D, Texture2DBuffer, TextureRange};
use rendiation_texture_packer::etagere_wrap::EtagerePacker;
use rendiation_texture_packer::{PackError, PackId, PackerConfig, RePackablePacker};

mod text;
pub use text::*;

mod fonts;
pub use fonts::*;

#[cfg(feature = "glyph_brush_impl")]
pub mod impls;

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
