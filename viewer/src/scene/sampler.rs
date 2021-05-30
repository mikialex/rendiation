use rendiation_texture::TextureSampler;

use super::BindableResource;

pub struct SceneSampler {
  sampler: TextureSampler,
  gpu: wgpu::Sampler,
}

impl BindableResource for SceneSampler {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::Sampler(&self.gpu)
  }
}
