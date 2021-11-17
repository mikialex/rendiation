mod pipeline;
use std::collections::HashMap;

use pipeline::*;
mod text_quad_instance;
use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use rendiation_webgpu::{GPUCommandEncoder, GPURenderPass, GPU};
use text_quad_instance::*;

use crate::{renderer::text_next::CacheQueuedResult, FontManager, TextInfo};

use super::text_next::{GlyphBrushLayouter, GlyphCache, TextCache, WebGPUTextureCache};

pub struct GPUxUITextPrimitive {
  vertex_buffer: wgpu::Buffer,
  length: u32,
}

pub struct TextRenderer {
  pipeline: TextRendererPipeline,
  texture_cache: WebGPUTextureCache,
  glyph_cache: GlyphCache,
  text_cache: TextCache,
}

impl TextRenderer {
  pub fn new(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
  ) -> Self {
    let init_size = Size::from_usize_pair_min_one((512, 512));
    let tolerance = Default::default();

    Self {
      pipeline: TextRendererPipeline::new(
        device,
        filter_mode,
        render_format,
        init_size,
        Vec2::new(1000., 1000.),
      ),
      glyph_cache: GlyphCache::new(init_size, tolerance),
      texture_cache: WebGPUTextureCache::init(init_size, device),
      text_cache: TextCache::new(GlyphBrushLayouter::default()),
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &wgpu::Queue) {
    self.pipeline.resize_view(size, queue)
  }

  pub fn draw_gpu_text<'a>(&'a self, pass: &mut GPURenderPass<'a>, text: &'a GPUxUITextPrimitive) {
    self.pipeline.draw(pass, text)
  }

  pub fn queue_text(&mut self, text: &TextInfo) {
    self.text_cache.queue(text);
  }

  pub fn get_cache_gpu_text(&self, text: &TextInfo) {
    //
  }

  pub fn process_queued(&mut self, gpu: &GPU, fonts: &FontManager) {
    self.text_cache.process_queued(&mut self.glyph_cache);

    match self
      .glyph_cache
      .process_queued(
        |data, range| {
          //
        },
        |new_size| {
          //
        },
        fonts,
      )
      .unwrap()
    {
      CacheQueuedResult::Adding => {
        // build only new queued text
      }
      CacheQueuedResult::Reordering => {
        // refresh all cached text with new glyph position
      }
    }
  }
}
