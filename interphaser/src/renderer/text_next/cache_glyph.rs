use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use std::collections::HashMap;

use crate::FontManager;

use super::{
  AbGlyphRaster, GlyphCacheResult, GlyphID, GlyphPacker, GlyphRaster, GlyphRasterInfo,
  GlyphRasterTolerance, NormalizedGlyphRasterInfo,
};

pub struct GlyphCache {
  packer: GlyphPacker,
  raster: Box<dyn GlyphRaster>,
  queue: HashMap<(GlyphID, NormalizedGlyphRasterInfo), GlyphRasterInfo>,
  current_size: Size,
  tolerance: GlyphRasterTolerance,
}

/// Successful method of caching of the queue.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CacheQueuedResult {
  /// Added any additional glyphs into the texture without affecting
  /// the position of any already cached glyphs in the latest queue.
  ///
  /// Glyphs not in the latest queue may have been removed.
  Adding,
  /// Fit the glyph queue by re-ordering all glyph texture positions.
  /// Previous texture positions are no longer valid.
  Reordering,
}

/// Returned from `DrawCache::cache_queued`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CacheWriteErr {
  /// Not all of the requested glyphs can fit into the cache, even if the
  /// cache is completely cleared before the attempt.
  NoRoomForWholeQueue,
}

pub enum TextureCacheAction<'a> {
  ResizeTo(Size),
  UpdateAt {
    data: &'a Texture2DBuffer<u8>,
    range: TextureRange,
  },
}

impl GlyphCache {
  pub fn new(init_size: Size, tolerance: GlyphRasterTolerance) -> Self {
    Self {
      packer: GlyphPacker::init(init_size),
      raster: Box::new(AbGlyphRaster {}),
      queue: Default::default(),
      current_size: init_size,
      tolerance,
    }
  }

  pub fn process_queued(
    &mut self,
    mut cache_update: impl FnMut(TextureCacheAction) -> bool, // return if cache_resize success
    fonts: &FontManager,
  ) -> Result<CacheQueuedResult, CacheWriteErr> {
    let mut failed_process_all = true;
    let mut previous_cache_invalid = false;

    let mut packer = self.packer.process_queued(&self.queue);

    'all_process: while failed_process_all {
      for (&(glyph_id, info), &info_raw) in self.queue.iter() {
        match packer.pack(glyph_id, info, info_raw, self.raster.as_mut(), fonts) {
          GlyphCacheResult::NewCached { result, data } => {
            cache_update(TextureCacheAction::UpdateAt {
              data: &data,
              range: result.1,
            });
          }
          GlyphCacheResult::AlreadyCached(_) => {}
          GlyphCacheResult::NotEnoughSpace => {
            let new_size = self.current_size * 2;

            if !cache_update(TextureCacheAction::ResizeTo(new_size)) {
              return Err(CacheWriteErr::NoRoomForWholeQueue);
            }
            packer.rebuild_all(new_size);

            failed_process_all = true;
            previous_cache_invalid = true;
            continue 'all_process;
          }
        }
      }
      failed_process_all = false;
    }

    self.queue.clear();

    Ok(match previous_cache_invalid {
      true => CacheQueuedResult::Reordering,
      false => CacheQueuedResult::Adding,
    })
  }

  pub fn queue_glyph(&mut self, glyph_id: GlyphID, info: GlyphRasterInfo) {
    self
      .queue
      .insert((glyph_id, info.normalize(&self.tolerance)), info);
  }
}
