pub mod pack_impl;

use rendiation_texture_core::{Size, TextureRange};

use crate::*;

#[derive(Debug, Clone, Copy)]
pub struct PackerConfig2d {
  pub allow_90_rotation: bool,
  pub full_size: Size,
}

impl Default for PackerConfig2d {
  fn default() -> Self {
    Self {
      allow_90_rotation: false,
      full_size: Size::from_usize_pair_min_one((512, 512)),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackResult2d {
  pub range: TextureRange,
  /// clockwise not matter, should agree with the outer implementation
  pub rotated: bool,
}
