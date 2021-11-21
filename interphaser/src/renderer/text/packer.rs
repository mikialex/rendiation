use glyph_brush::FontId;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use rendiation_texture_packer::{PackError, PackId, PackerConfig, RePackablePacker};

use crate::FontManager;

use super::{GlyphRaster, GlyphRasterInfo, NormalizedGlyphRasterInfo};

pub struct GlyphPacker {
  packer: Box<dyn RePackablePacker>,
  pack_info: LinkedHashMap<(GlyphID, NormalizedGlyphRasterInfo), (PackId, TextureRange)>,
}

impl GlyphPacker {
  pub fn init(init_size: Size, mut packer: impl RePackablePacker + 'static) -> Self {
    packer.config(PackerConfig {
      allow_90_rotation: false,
      init_size,
    });
    Self {
      packer: Box::new(packer),
      pack_info: Default::default(),
    }
  }

  pub fn re_init(&mut self, init_size: Size) {
    self.packer.config(PackerConfig {
      allow_90_rotation: false,
      init_size,
    });
    self.pack_info = Default::default();
  }

  pub fn process_queued<'a>(
    &'a mut self,
    queue: &'a HashMap<(GlyphID, NormalizedGlyphRasterInfo), GlyphRasterInfo>,
  ) -> GlyphPackFrameTask<'a> {
    GlyphPackFrameTask {
      packer: self,
      queue,
    }
  }
}

pub struct GlyphPackFrameTask<'a> {
  packer: &'a mut GlyphPacker,
  queue: &'a HashMap<(GlyphID, NormalizedGlyphRasterInfo), GlyphRasterInfo>,
}

impl<'a> GlyphPackFrameTask<'a> {
  pub fn rebuild_all(&mut self, new_size: Size) {
    self.packer.re_init(new_size);
  }

  pub fn pack(
    &mut self,
    glyph_id: GlyphID,
    info: NormalizedGlyphRasterInfo,
    raw_info: GlyphRasterInfo,
    raster: &mut dyn GlyphRaster,
    fonts: &FontManager,
  ) -> GlyphAddCacheResult {
    if let Some(result) = self.packer.pack_info.get_refresh(&(glyph_id, info)) {
      GlyphAddCacheResult::AlreadyCached(*result)
    } else {
      let data = raster.raster(glyph_id, raw_info, fonts);

      loop {
        match self.packer.packer.pack_with_id(data.size()) {
          Ok(result) => {
            let range = result.result.range;

            let result = *self
              .packer
              .pack_info
              .entry((glyph_id, info))
              .or_insert((result.id, range));

            break GlyphAddCacheResult::NewCached { result, data };
          }
          Err(err) => match err {
            PackError::SpaceNotEnough => {
              if let Some((k, _)) = self.packer.pack_info.back() {
                if self.queue.contains_key(k) {
                  break GlyphAddCacheResult::NotEnoughSpace;
                } else {
                  let (_, v) = self.packer.pack_info.pop_back().unwrap();
                  self.packer.packer.unpack(v.0).expect("glyph unpack error");
                }
              } else {
                break GlyphAddCacheResult::NotEnoughSpace;
              }
            }
          },
        }
      }
    }
  }
}

pub enum GlyphAddCacheResult {
  NewCached {
    result: (PackId, TextureRange),
    data: Texture2DBuffer<u8>,
  },
  AlreadyCached((PackId, TextureRange)),
  NotEnoughSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphID(pub char, pub FontId);
