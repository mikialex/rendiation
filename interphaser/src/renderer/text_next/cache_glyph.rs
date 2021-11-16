use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use std::collections::HashSet;

use crate::FontManager;

use super::{
  AbGlyphRaster, GlyphCacheResult, GlyphID, GlyphPacker, GlyphRaster, GlyphRasterInfo,
  GlyphRasterTolerance, NormalizedGlyphRasterInfo,
};

pub struct GlyphCache {
  packer: GlyphPacker,
  raster: Box<dyn GlyphRaster>,
  queue: HashSet<(GlyphID, NormalizedGlyphRasterInfo)>,
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
  /// At least one of the queued glyphs is too big to fit into the cache, even
  /// if all other glyphs are removed.
  GlyphTooLarge,
  /// Not all of the requested glyphs can fit into the cache, even if the
  /// cache is completely cleared before the attempt.
  NoRoomForWholeQueue,
}

impl GlyphCache {
  pub fn new(init_size: Size) -> Self {
    Self {
      packer: GlyphPacker::init(init_size),
      raster: Box::new(AbGlyphRaster {}),
      queue: Default::default(),
      current_size: init_size,
      tolerance: Default::default(),
    }
  }

  pub fn process_queued(
    &mut self,
    mut cache_update: impl FnMut(&Texture2DBuffer<u8>, TextureRange),
    mut cache_resize: impl FnMut(Size),
    fonts: &FontManager,
  ) -> Result<CacheQueuedResult, CacheWriteErr> {
    let mut failed_process_all = true;
    let mut previous_cache_invalid = false;

    let mut packer = self.packer.process_queued(&self.queue);

    'all_process: while failed_process_all {
      for &(glyph_id, info) in self.queue.iter() {
        match packer.pack(glyph_id, info, self.raster.as_mut(), fonts) {
          GlyphCacheResult::NewCached { result, data } => {
            cache_update(&data, result.1);
          }
          GlyphCacheResult::AlreadyCached(_) => {}
          GlyphCacheResult::NotEnoughSpace => {
            let new_size = self.current_size * 2;
            // todo max size limit

            cache_resize(new_size);
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
      .insert((glyph_id, info.normalize(&self.tolerance)));
  }
}
