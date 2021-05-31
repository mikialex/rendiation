use rendiation_texture::TextureSampler;

use super::{BindableResource, SamplerHandle, Scene};

pub struct SceneSampler {
  sampler: TextureSampler,
  gpu: Option<wgpu::Sampler>,
}

impl SceneSampler {
  pub fn update(&mut self, device: &wgpu::Device) {
    self.gpu.get_or_insert_with(|| {
      todo!()
      // device.create_sampler(&wgpu::SamplerDescriptor {
      //   label: None,
      //   address_mode_u: (),
      //   address_mode_v: (),
      //   address_mode_w: (),
      //   mag_filter: (),
      //   min_filter: (),
      //   mipmap_filter: (),
      //   ..Default::default()
      // })
    });
  }
}

impl BindableResource for wgpu::Sampler {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::Sampler(self)
  }
}

impl Scene {
  pub fn add_sampler(&mut self, sampler: TextureSampler) -> SamplerHandle {
    self.samplers.insert(SceneSampler { sampler, gpu: None })
  }
}
