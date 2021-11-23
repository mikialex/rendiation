use std::collections::HashMap;

use rendiation_texture::{Size, TextureRange};
use rendiation_webgpu::{
  util::DeviceExt, WebGPUTexture2d, WebGPUTexture2dDescriptor, WebGPUTexture2dSource,
};

use super::{TextHash, TextQuadInstance};

pub struct WebGPUxTextPrimitive {
  pub vertex_buffer: wgpu::Buffer,
  pub length: u32,
}

pub fn create_gpu_text(
  device: &wgpu::Device,
  instances: &[TextQuadInstance],
) -> Option<WebGPUxTextPrimitive> {
  if instances.is_empty() {
    return None;
  }
  let instances_bytes = bytemuck::cast_slice(instances);

  let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: None,
    contents: instances_bytes,
    usage: wgpu::BufferUsages::VERTEX,
  });

  WebGPUxTextPrimitive {
    vertex_buffer,
    length: instances.len() as u32,
  }
  .into()
}

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
  cached: HashMap<TextHash, WebGPUxTextPrimitive>,
}

impl WebGPUTextCache {
  pub fn get_cache(&self, text: TextHash) -> Option<&WebGPUxTextPrimitive> {
    self.cached.get(&text)
  }

  pub fn drop_cache(&mut self, text: TextHash) {
    self.cached.remove(&text);
  }

  pub fn add_cache(&mut self, text: TextHash, data: WebGPUxTextPrimitive) {
    self.cached.insert(text, data);
  }

  pub fn clear_cache(&mut self) {
    self.cached.clear()
  }
}
