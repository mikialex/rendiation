mod gpu_renderer;
use gpu_renderer::*;
mod gpu_cache;
pub use gpu_cache::*;
use rendiation_algebra::Vec2;
use rendiation_texture::{Size, Texture2DBuffer, TextureFormat};

use crate::*;
use crate::{FontManager, TextCache, TextHash, TextureCacheAction, VertexCacheAction};

pub struct TextRenderer {
  renderer: TextWebGPURenderer,
  gpu_texture_cache: WebGPUTextureCache,
  gpu_vertex_cache: WebGPUTextCache,
}

impl TextRenderer {
  pub fn new(
    device: &GPUDevice,
    filter_mode: FilterMode,
    render_format: TextureFormat,
    init_size: Size,
  ) -> Self {
    let texture_cache = WebGPUTextureCache::init(init_size, device);

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
    }
  }

  pub fn resize_view(&mut self, size: Vec2<f32>, queue: &Queue) {
    self.renderer.resize_view(size, queue)
  }

  pub fn draw_gpu_text<'a>(&'a self, pass: &mut GPURenderPass<'a>, text: TextHash) {
    if let Some(gpu_text) = self.gpu_vertex_cache.get_cache(text) {
      self.renderer.draw(pass, gpu_text)
    }
  }

  pub fn process_queued(&mut self, gpu: &GPU, fonts: &FontManager, cache: &mut TextCache) {
    cache.process_queued(
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
            .update_texture(&TextureBufferSource { data }, range, &gpu.queue);
          true
        }
      },
      |action| match action {
        VertexCacheAction::Add { hash, data } => {
          if let Some(text) = create_gpu_text(&gpu.device, data.as_slice()) {
            self.gpu_vertex_cache.add_cache(hash, text);
          }
        }
        VertexCacheAction::Remove(hash) => self.gpu_vertex_cache.drop_cache(hash),
      },
    );
  }
}

struct TextureBufferSource<'a> {
  data: &'a Texture2DBuffer<u8>,
}

impl<'a> WebGPU2DTextureSource for TextureBufferSource<'a> {
  fn format(&self) -> TextureFormat {
    TextureFormat::R8Unorm
  }

  fn as_bytes(&self) -> &[u8] {
    self.data.as_byte_buffer()
  }

  fn size(&self) -> Size {
    self.data.size()
  }
}
