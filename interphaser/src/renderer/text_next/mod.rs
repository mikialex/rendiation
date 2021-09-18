use std::collections::HashMap;

use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use rendiation_texture_packer::BaseTexturePacker;

// https://github.com/alexheretic/glyph-brush/blob/master/draw-cache/src/lib.rs

pub struct GlyphCache {
  raster: Box<dyn GlyphRaster>,
  packer: Box<dyn BaseTexturePacker>,
  pack_info: HashMap<GlyphID, GlyphCacheInfo>,
}

impl GlyphCache {
  pub fn init_with_default_size(init_size: Size) -> Self {
    // Self {
    //   cache_bitmap: vec![0; width * height],
    //   cache_size: init_size,
    // }
    todo!()
  }

  pub fn rebuild(&mut self, size: Size) {
    todo!()
  }

  pub fn add_cache(&mut self, glyph_id: GlyphID) -> GlyphCacheResult {
    if let Some(pre_cached) = self.pack_info.get(&glyph_id) {
      return GlyphCacheResult::AlreadyCached(pre_cached);
    }

    todo!()
  }

  pub fn drop_cache(&mut self, glyph_id: GlyphID) {
    todo!()
  }
}

pub enum GlyphCacheResult<'a> {
  NewCached {
    result: &'a GlyphCacheInfo,
    data: Texture2DBuffer<u8>,
  },
  AlreadyCached(&'a GlyphCacheInfo),
  NotEnoughSpace,
}

pub trait GlyphRaster {
  fn raster(&mut self, glyph_id: GlyphID);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphID(usize);

pub struct GlyphCacheInfo {
  self_id: GlyphID,
  cache_at: TextureRange,
}

pub struct GlyphRasterTolerance {
  pub scale: f32,
  pub position: f32,
}
