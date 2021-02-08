use crate::WGPURenderer;

/// webgpu sampler wrapper
pub struct WGPUSampler {
  gpu_sampler: wgpu::Sampler,
  descriptor: wgpu::SamplerDescriptor<'static>,
}

impl WGPUSampler {
  pub fn default(renderer: &WGPURenderer) -> Self {
    WGPUSamplerBuilder::new().build(renderer)
  }

  pub fn get_gpu_sampler(&self) -> &wgpu::Sampler {
    &self.gpu_sampler
  }
  pub fn get_descriptor(&self) -> &wgpu::SamplerDescriptor {
    &self.descriptor
  }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureWrapping {
  ClampToEdge,
  Repeat,
  MirrorRepeat,
}

impl TextureWrapping {
  pub fn to_wgpu(&self) -> wgpu::AddressMode {
    match self {
      TextureWrapping::ClampToEdge => wgpu::AddressMode::ClampToEdge,
      TextureWrapping::Repeat => wgpu::AddressMode::Repeat,
      TextureWrapping::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
    }
  }
}

pub struct WGPUSamplerBuilder {
  descriptor: wgpu::SamplerDescriptor<'static>,
}

impl AsMut<Self> for WGPUSamplerBuilder {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl Default for WGPUSamplerBuilder {
  fn default() -> Self {
    Self::new()
  }
}

impl WGPUSamplerBuilder {
  pub fn new() -> Self {
    Self {
      descriptor: wgpu::SamplerDescriptor {
        label: None,
        anisotropy_clamp: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: -100.0,
        lod_max_clamp: 100.0,
        compare: None,
        border_color: None,
      },
    }
  }

  pub fn wrapping_u(&mut self, value: TextureWrapping) -> &mut Self {
    self.descriptor.address_mode_u = value.to_wgpu();
    self
  }

  pub fn wrapping_v(&mut self, value: TextureWrapping) -> &mut Self {
    self.descriptor.address_mode_u = value.to_wgpu();
    self
  }

  pub fn wrapping_w(&mut self, value: TextureWrapping) -> &mut Self {
    self.descriptor.address_mode_u = value.to_wgpu();
    self
  }

  pub fn build(self, renderer: &WGPURenderer) -> WGPUSampler {
    let sampler = renderer.device.create_sampler(&self.descriptor);

    WGPUSampler {
      gpu_sampler: sampler,
      descriptor: self.descriptor,
    }
  }
}
