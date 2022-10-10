use std::collections::HashMap;

use rendiation_texture::{Size, TextureRange};
use webgpu::{
  map_size_gpu, util::DeviceExt, GPU2DTexture, GPU2DTextureView, GPUTexture, WebGPU2DTextureSource,
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
  texture: GPU2DTexture,
  view: GPU2DTextureView,
}

impl WebGPUTextureCache {
  pub fn init(size: Size, device: &webgpu::GPUDevice) -> Self {
    let desc = webgpu::TextureDescriptor {
      label: "text-glyph-atlas".into(),
      size: map_size_gpu(size),
      dimension: webgpu::TextureDimension::D2,
      format: webgpu::TextureFormat::R8Unorm,
      usage: webgpu::TextureUsages::TEXTURE_BINDING | webgpu::TextureUsages::COPY_DST,
      mip_level_count: 1,
      sample_count: 1,
    };

    let texture = GPUTexture::create(desc, device);
    let texture: GPU2DTexture = texture.try_into().unwrap();
    let view = texture.create_view(Default::default()).try_into().unwrap();

    Self { texture, view }
  }
  pub fn update_texture(
    &self,
    data: &dyn WebGPU2DTextureSource,
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
