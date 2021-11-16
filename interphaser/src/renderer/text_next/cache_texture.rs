use rendiation_texture::{Size, TextureRange};
use rendiation_webgpu::{WebGPUTexture2d, WebGPUTexture2dDescriptor, WebGPUTexture2dSource};

pub struct WebGPUTextureCache {
  sampler: wgpu::Sampler,
  texture: WebGPUTexture2d,
}

impl WebGPUTextureCache {
  pub fn init(size: Size, device: &wgpu::Device) -> Self {
    let desc = WebGPUTexture2dDescriptor::from_size(size).with_format(wgpu::TextureFormat::R8Unorm);
    Self {
      sampler: device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
      }),
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
}
