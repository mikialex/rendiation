pub struct WGPUSampler {
  gpu_sampler: wgpu::Sampler,
  descriptor: wgpu::SamplerDescriptor,
}

impl WGPUSampler {
  pub fn new(device: &wgpu::Device) -> Self {
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
    let sampler = device.create_sampler(&des);

    Self {
      gpu_sampler: sampler,
      descriptor: des,
    }
  }

  pub fn get_gpu_sampler(&self) -> &wgpu::Sampler {
    &self.gpu_sampler
  }
}
