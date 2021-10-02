use std::collections::HashSet;

use rendiation_texture::{Size, Texture2DBuffer, TextureRange};
use rendiation_webgpu::{WebGPUTexture2d, WebGPUTexture2dSource, GPU};

use crate::FontManager;

use super::{GlyphCacheResult, GlyphID, GlyphPacker, GlyphRaster, GlyphRasterInfo};

pub struct GPUGlyphCache {
  gpu: WebGPUGlyphCacheInstance,
  packer: GlyphPacker,
  raster: Box<dyn GlyphRaster>,
  fonts: FontManager,
  queue: HashSet<(GlyphID, GlyphRasterInfo)>,
  active_glyphs: HashSet<(GlyphID, GlyphRasterInfo), TextureRange>,
  current_size: Size,
}

struct WebGPUGlyphCacheInstance {
  sampler: wgpu::Sampler,
  texture: WebGPUTexture2d,
}

impl WebGPUGlyphCacheInstance {
  pub fn init(size: Size, device: &wgpu::Device) -> Self {
    Self {
      sampler: device.create_sampler(todo!()),
      texture: WebGPUTexture2d::create(device, todo!()),
    }
  }
  pub fn update_texture(
    &self,
    data: &dyn WebGPUTexture2dSource,
    range: TextureRange,
    queue: &wgpu::Queue,
  ) {
    self
      .texture
      .upload_with_origin(queue, data, 0, range.origin);
  }
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

impl GPUGlyphCache {
  pub fn process_queued(&mut self, gpu: &GPU) -> Result<CacheQueuedResult, CacheWriteErr> {
    let mut failed_process_all = true;
    let mut previous_cache_invalid = false;

    while failed_process_all {
      for &(glyph_id, info) in self.queue.iter() {
        match self.packer.pack(glyph_id, info, self.raster.as_mut()) {
          GlyphCacheResult::NewCached { result, data } => {
            // self.active_glyphs.insert((glyph_id, info), result);
            self.gpu.update_texture(&data, *result, &gpu.queue);
          }
          GlyphCacheResult::AlreadyCached(result) => {}
          GlyphCacheResult::NotEnoughSpace => {
            let new_size = self.current_size * 2;

            self.gpu = WebGPUGlyphCacheInstance::init(new_size, &gpu.device);
            self.packer = GlyphPacker::init(new_size);

            failed_process_all = true;
            previous_cache_invalid = true;
            self.active_glyphs.clear();
            break;
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
    self.queue.insert((glyph_id, info));
  }
}
