use std::collections::{hash_map::Entry, HashMap};

use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use rendiation_texture_packer::{skyline::SkylinePacker, PackError, PackerConfig, TexturePacker};

use super::{GlyphRaster, GlyphRasterInfo, NormalizedGlyphRasterInfo};

pub struct GlyphPacker {
  packer: Box<dyn TexturePacker>,
  pack_info: HashMap<GlyphID, TextureRange>,
}

impl GlyphPacker {
  pub fn init(init_size: Size) -> Self {
    Self {
      packer: Box::new(SkylinePacker::new(PackerConfig {
        allow_90_rotation: false,
        init_size,
      })),
      pack_info: Default::default(),
    }
  }

  pub fn pack(
    &mut self,
    glyph_id: GlyphID,
    info: NormalizedGlyphRasterInfo,
    raster: &mut dyn GlyphRaster,
  ) -> GlyphCacheResult {
    match self.pack_info.entry(glyph_id) {
      Entry::Occupied(entry) => GlyphCacheResult::AlreadyCached(entry.into_mut()),
      Entry::Vacant(entry) => {
        let data = raster.raster(glyph_id, info);

        match self.packer.pack(data.size()) {
          Ok(result) => {
            let result = result.range;

            let result = entry.insert(result);

            GlyphCacheResult::NewCached { result, data }
          }
          Err(err) => match err {
            PackError::SpaceNotEnough => GlyphCacheResult::NotEnoughSpace,
          },
        }
      }
    }
  }
}

pub enum GlyphCacheResult<'a> {
  NewCached {
    result: &'a TextureRange,
    data: Texture2DBuffer<u8>,
  },
  AlreadyCached(&'a TextureRange),
  NotEnoughSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphID(usize);
