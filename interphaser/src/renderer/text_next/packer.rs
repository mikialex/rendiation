use linked_hash_map::{Entry, LinkedHashMap};
use std::collections::HashSet;

use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use rendiation_texture_packer::{
  shelf::ShelfPacker, PackError, PackId, PackerConfig, RePackablePacker,
};

use super::{GlyphRaster, NormalizedGlyphRasterInfo};

pub struct GlyphPacker {
  packer: Box<dyn RePackablePacker>,
  pack_info: LinkedHashMap<(GlyphID, NormalizedGlyphRasterInfo), (PackId, TextureRange)>,
}

impl GlyphPacker {
  pub fn init(init_size: Size) -> Self {
    Self {
      packer: Box::new(ShelfPacker::new(PackerConfig {
        allow_90_rotation: false,
        init_size,
      })),
      pack_info: Default::default(),
    }
  }

  pub fn process_queued<'a>(
    &'a mut self,
    queue: &'a HashSet<(GlyphID, NormalizedGlyphRasterInfo)>,
  ) -> GlyphPackFrameTask<'a> {
    GlyphPackFrameTask {
      packer: self,
      queue,
    }
  }
}

pub struct GlyphPackFrameTask<'a> {
  packer: &'a mut GlyphPacker,
  queue: &'a HashSet<(GlyphID, NormalizedGlyphRasterInfo)>,
}

impl<'a> GlyphPackFrameTask<'a> {
  pub fn rebuild_all(&mut self, new_size: Size) {
    *self.packer = GlyphPacker::init(new_size);
  }

  pub fn pack(
    &mut self,
    glyph_id: GlyphID,
    info: NormalizedGlyphRasterInfo,
    raster: &mut dyn GlyphRaster,
  ) -> GlyphCacheResult {
    if let Some(result) = self.packer.pack_info.get_refresh(&(glyph_id, info)) {
      GlyphCacheResult::AlreadyCached(*result)
    } else {
      let data = raster.raster(glyph_id, info);

      loop {
        match self.packer.packer.pack_with_id(data.size()) {
          Ok(result) => {
            let range = result.result.range;

            let result = *self
              .packer
              .pack_info
              .entry((glyph_id, info))
              .or_insert((result.id, range));

            break GlyphCacheResult::NewCached { result, data };
          }
          Err(err) => match err {
            PackError::SpaceNotEnough => {
              if let Some((k, _)) = self.packer.pack_info.back() {
                if self.queue.contains(k) {
                  break GlyphCacheResult::NotEnoughSpace;
                } else {
                  let (_, v) = self.packer.pack_info.pop_back().unwrap();
                  self.packer.packer.un_pack(v.0);
                }
              } else {
                break GlyphCacheResult::NotEnoughSpace;
              }
            }
          },
        }
      }
    }
  }
}

pub enum GlyphCacheResult {
  NewCached {
    result: (PackId, TextureRange),
    data: Texture2DBuffer<u8>,
  },
  AlreadyCached((PackId, TextureRange)),
  NotEnoughSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphID(usize);
