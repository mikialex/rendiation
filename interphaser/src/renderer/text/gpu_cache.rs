use rendiation_texture::{Size, TextureRange};

use crate::*;

pub struct WebGPUxTextPrimitive {
  pub vertex_buffer: Buffer,
  pub length: u32,
}

pub fn create_gpu_text(
  device: &Device,
  instances: &[TextQuadInstance],
) -> Option<WebGPUxTextPrimitive> {
  if instances.is_empty() {
    return None;
  }
  let instances_bytes = bytemuck::cast_slice(instances);

  let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
    label: None,
    contents: instances_bytes,
    usage: BufferUsages::VERTEX,
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
  pub fn init(size: Size, device: &GPUDevice) -> Self {
    let desc = TextureDescriptor {
      label: "text-glyph-atlas".into(),
      size: map_size_gpu(size),
      dimension: TextureDimension::D2,
      format: TextureFormat::R8Unorm,
      usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
      mip_level_count: 1,
      sample_count: 1,
      view_formats: &[] as &'static [rendiation_texture::TextureFormat],
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
    queue: &Queue,
  ) {
    self
      .texture
      .upload_with_origin(queue, data, 0, range.origin);
  }

  pub fn get_view(&self) -> &TextureView {
    &self.view.0
  }
}

#[derive(Default)]
pub struct WebGPUTextCache {
  cached: FastHashMap<TextHash, WebGPUxTextPrimitive>,
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
