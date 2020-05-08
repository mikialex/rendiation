use crate::WGPURenderer;

/// webgpu sampler wrapper
pub struct WGPUSampler {
  gpu_sampler: wgpu::Sampler,
  descriptor: wgpu::SamplerDescriptor,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureWrapping {
  ClampToEdge,
  Repeat,
  MirrorRepeat,
}

pub struct WGPUSamplerBuilder {
  wrapping_u: TextureWrapping,
  wrapping_v: TextureWrapping,
  wrapping_w: TextureWrapping,
}

impl WGPUSamplerBuilder {
  pub fn new() -> Self {
    Self {
      wrapping_u: TextureWrapping::ClampToEdge,
      wrapping_v: TextureWrapping::ClampToEdge,
      wrapping_w: TextureWrapping::ClampToEdge,
    }
  }

  pub fn wrapping_u(mut self: Self, value: TextureWrapping) -> Self {
    self.wrapping_u = value;
    self
  }

  pub fn wrapping_v(mut self: Self, value: TextureWrapping) -> Self {
    self.wrapping_v = value;
    self
  }

  pub fn wrapping_w(mut self: Self, value: TextureWrapping) -> Self {
    self.wrapping_w = value;
    self
  }

  pub fn build() -> WGPUSampler {
    todo!()
  }
}

impl WGPUSampler {
  pub fn new(renderer: &WGPURenderer) -> Self {
    let des = wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Linear,
      mipmap_filter: wgpu::FilterMode::Nearest,
      lod_min_clamp: -100.0,
      lod_max_clamp: 100.0,
      compare_function: wgpu::CompareFunction::Always,
    };
    let sampler = renderer.device.create_sampler(&des);

    Self {
      gpu_sampler: sampler,
      descriptor: des,
    }
  }

  pub fn get_gpu_sampler(&self) -> &wgpu::Sampler {
    &self.gpu_sampler
  }
}
