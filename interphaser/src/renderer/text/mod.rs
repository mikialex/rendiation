mod gpu_renderer;

use gpu_renderer::*;
use rendiation_algebra::Vec2;
use rendiation_texture::Size;
use rendiation_webgpu::{GPURenderPass, GPU};

pub mod cache_glyph;
pub use cache_glyph::*;

pub mod cache_text;
pub use cache_text::*;

pub mod gpu_cache;
pub use gpu_cache::*;

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
  renderer: TextWebGPURenderer,
  gpu_texture_cache: WebGPUTextureCache,
  gpu_vertex_cache: WebGPUTextCache,

  cache: TextCache,
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

    let glyph_cache = GlyphCache::new(init_size, tolerance);

    let text_cache = TextCache::new(glyph_cache, GlyphBrushLayouter::default());

    Self {
      renderer: TextWebGPURenderer::new(
        device,
        filter_mode,
        render_format,
        Vec2::new(1000., 1000.),
        texture_cache.get_view(),
      ),
      gpu_texture_cache: texture_cache,
      gpu_vertex_cache: Default::default(),
      cache: text_cache,
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &wgpu::Queue) {
    self.renderer.resize_view(size, queue)
  }

  pub fn draw_gpu_text<'a>(&'a self, pass: &mut GPURenderPass<'a>, text: &'a GPUxUITextPrimitive) {
    self.renderer.draw(pass, text)
  }

  pub fn queue_text(&mut self, text: &TextInfo) {
    self.cache.queue(text);
  }

  pub fn get_cache_gpu_text(&self, text: &TextInfo) -> Option<GPUxUITextPrimitive> {
    todo!();
  }

  pub fn process_queued(&mut self, gpu: &GPU, fonts: &FontManager) {
    self.cache.process_queued(
      fonts,
      |action| match action {
        TextureCacheAction::ResizeTo(new_size) => {
          if usize::from(new_size.width) > 4096 || usize::from(new_size.height) > 4096 {
            return false;
          }
          let device = &gpu.device;
          self.gpu_texture_cache = WebGPUTextureCache::init(new_size, device);
          self
            .renderer
            .cache_resized(device, self.gpu_texture_cache.get_view());
          true
        }
        TextureCacheAction::UpdateAt { data, range } => {
          self
            .gpu_texture_cache
            .update_texture(data, range, &gpu.queue);
          true
        }
      },
      |hash, data| {
        //
      },
    );
  }
}
