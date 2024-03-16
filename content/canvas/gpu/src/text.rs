use crate::*;

struct TextRenderer {
  atlas_view: GPU2DTextureView,
  sampler: webgpu::Sampler,
  vertex_cache: HashMap<u32, Vec>,
}

impl TextWriterExtensionAPI for TextRenderer {
  fn write_text(&mut self, layouted_text: u32) {
    todo!()
  }
}

impl TextRenderer {
  pub fn update(&mut self, manager: &mut TextCache) {
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
