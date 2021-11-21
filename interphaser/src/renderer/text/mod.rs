mod renderer;

use renderer::*;
pub mod text_quad_instance;
use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use rendiation_webgpu::{GPURenderPass, GPU};

pub mod cache_glyph;
pub use cache_glyph::*;

pub mod cache_text;
pub use cache_text::*;

pub mod cache_texture;
pub use cache_texture::*;

pub mod layout;
pub use layout::*;

pub mod raster;
pub use raster::*;

pub mod packer;
pub use packer::*;

use crate::{FontManager, TextInfo};

pub struct GPUxUITextPrimitive {
  vertex_buffer: wgpu::Buffer,
  length: u32,
}

pub struct TextRenderer {
  renderer: TextRendererGPURenderer,
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

    let texture_cache = WebGPUTextureCache::init(init_size, device);

    Self {
      renderer: TextRendererGPURenderer::new(
        device,
        filter_mode,
        render_format,
        Vec2::new(1000., 1000.),
        texture_cache.get_view(),
      ),
      glyph_cache: GlyphCache::new(init_size, tolerance),
      texture_cache,
      text_cache: TextCache::new(GlyphBrushLayouter::default()),
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &wgpu::Queue) {
    self.renderer.resize_view(size, queue)
  }

  pub fn draw_gpu_text<'a>(&'a self, pass: &mut GPURenderPass<'a>, text: &'a GPUxUITextPrimitive) {
    self.renderer.draw(pass, text)
  }

  pub fn queue_text(&mut self, text: &TextInfo) {
    self.text_cache.queue(text);
  }

  pub fn get_cache_gpu_text(&self, text: &TextInfo) {
    //
  }

  pub fn process_queued(&mut self, gpu: &GPU, fonts: &FontManager) {
    self.text_cache.process_queued(&mut self.glyph_cache, fonts);

    match self
      .glyph_cache
      .process_queued(
        |action| match action {
          TextureCacheAction::ResizeTo(new_size) => {
            if usize::from(new_size.width) > 4096 || usize::from(new_size.height) > 4096 {
              return false;
            }
            let device = &gpu.device;
            self.texture_cache = WebGPUTextureCache::init(new_size, device);
            self
              .renderer
              .cache_resized(device, self.texture_cache.get_view());
            true
          }
          TextureCacheAction::UpdateAt { data, range } => {
            self.texture_cache.update_texture(data, range, &gpu.queue);
            true
          }
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
