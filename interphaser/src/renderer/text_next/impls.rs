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
    queue: wgpu::Queue,
  ) {
    self
      .texture
      .upload_with_origin(&queue, data, 0, range.origin);
  }
}

impl GPUGlyphCache {
  pub fn process_queued(&mut self, gpu: &GPU) {
    let mut failed_process_all = true;

    while failed_process_all {
      for &(glyph_id, info) in self.queue.iter() {
        match self.packer.queue(glyph_id, info, self.raster.as_mut()) {
          GlyphCacheResult::NewCached { result, data } => {}
          GlyphCacheResult::AlreadyCached(result) => {}
          GlyphCacheResult::NotEnoughSpace => {
            let new_size = self.current_size * 2;

            self.gpu = WebGPUGlyphCacheInstance::init(new_size, &gpu.device);
            self.packer = GlyphPacker::init(new_size);

            failed_process_all = true;
            break;
          }
        }
      }
      failed_process_all = false;
    }

    //
  }
  pub fn queue_glyph(&mut self, glyph_id: GlyphID, info: GlyphRasterInfo) {
    self.queue.insert((glyph_id, info));
  }
}
