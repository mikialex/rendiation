use std::collections::HashMap;

use rendiation_texture::{Size, TextureRange};
use webgpu::{
  util::DeviceExt, GPUTexture2d, GPUTexture2dView, WebGPUTexture2dDescriptor, WebGPUTexture2dSource,
};

use crate::{TextHash, TextQuadInstance};

pub struct WebGPUxTextPrimitive {
  pub vertex_buffer: webgpu::Buffer,
  pub length: u32,
}

pub fn create_gpu_text(
  device: &webgpu::Device,
  instances: &[TextQuadInstance],
) -> Option<WebGPUxTextPrimitive> {
  if instances.is_empty() {
    return None;
  }
  let instances_bytes = bytemuck::cast_slice(instances);

  let vertex_buffer = device.create_buffer_init(&webgpu::util::BufferInitDescriptor {
    label: None,
    contents: instances_bytes,
    usage: webgpu::BufferUsages::VERTEX,
  });

  WebGPUxTextPrimitive {
    vertex_buffer,
    length: instances.len() as u32,
  }
  .into()
}

pub struct WebGPUTextureCache {
  texture: GPUTexture2d,
  view: GPUTexture2dView,
}

impl WebGPUTextureCache {
  pub fn init(size: Size, device: &webgpu::Device) -> Self {
    let desc = WebGPUTexture2dDescriptor::from_size(size) //
      .with_format(webgpu::TextureFormat::R8Unorm);

    let texture = GPUTexture2d::create(desc, device);
    let view = texture.create_view((), device);

    Self { texture, view }
  }
  pub fn update_texture(
    &self,
    data: &dyn WebGPUTexture2dSource,
    range: TextureRange,
    queue: &webgpu::Queue,
  ) {
    self
      .texture
      .upload_with_origin(queue, data, 0, range.origin);
  }

  pub fn get_view(&self) -> &webgpu::TextureView {
    &self.view.0
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
