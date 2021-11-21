use std::collections::HashMap;

use rendiation_texture::{Size, TextureRange};
use rendiation_webgpu::{WebGPUTexture2d, WebGPUTexture2dDescriptor, WebGPUTexture2dSource};

use super::{GPUxUITextPrimitive, TextHash};

pub struct WebGPUTextureCache {
  texture: WebGPUTexture2d,
}

impl WebGPUTextureCache {
  pub fn init(size: Size, device: &wgpu::Device) -> Self {
    let desc = WebGPUTexture2dDescriptor::from_size(size).with_format(wgpu::TextureFormat::R8Unorm);
    Self {
      texture: WebGPUTexture2d::create(device, desc),
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

  pub fn get_view(&self) -> &wgpu::TextureView {
    self.texture.get_default_view()
  }
}

#[derive(Default)]
pub struct WebGPUTextCache {
  cached: HashMap<TextHash, GPUxUITextPrimitive>,
}

impl WebGPUTextCache {
  pub fn drop_cache(&mut self, text: TextHash) {
    self.cached.remove(&text);
  }

  pub fn clear_cache(&mut self) {
    self.cached.clear()
  }
}
