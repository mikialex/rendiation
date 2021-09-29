use std::collections::HashMap;

use linked_hash_map::LinkedHashMap;
use rendiation_texture::TextureRange;

use crate::*;

pub struct ShelfPacker {
  config: PackerConfig,
  rows: LinkedHashMap<u32, Row>,
  /// Mapping of row gaps bottom -> top
  space_start_for_end: HashMap<u32, u32>,
  /// Mapping of row gaps top -> bottom
  space_end_for_start: HashMap<u32, u32>,
}

impl ShelfPacker {
  pub fn new(config: PackerConfig) -> Self {
    todo!()
  }
}

/// Row of pixel data
struct Row {
  /// Row pixel height
  height: usize,
  /// Pixel width current in use by glyphs
  width: usize,

  items: Vec<TextureRange>,
}

impl BaseTexturePacker for ShelfPacker {
  fn config(&mut self, config: PackerConfig) {
    self.config = config;
    self.reset();
  }

  fn reset(&mut self) {
    *self = Self::new(self.config)
  }
}

/// https://github.com/alexheretic/glyph-brush/blob/master/draw-cache/src/lib.rs
impl RePackablePacker for ShelfPacker {
  fn pack_with_id(
    &mut self,
    input: rendiation_texture::Size,
  ) -> Result<PackResultWithId, PackError> {
    // todo check input can contained in all;

    // find row to put the glyph in, most used rows first
    let row = self.rows.iter().find(|(_, row)| {
      row.width >= usize::from(input.width) && row.height >= usize::from(input.height)
    });

    todo!()
  }

  fn un_pack(&mut self, id: PackId) {
    todo!()
  }
}
