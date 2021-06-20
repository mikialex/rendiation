use rendiation_texture::{AddressMode, FilterMode, TextureSampler};

use crate::renderer::BindableResource;

use super::{MaterialBindableItemPair, MaterialBindableResourceUpdate, SamplerHandle, Scene};

impl MaterialBindableResourceUpdate for TextureSampler {
  type GPU = wgpu::Sampler;
  fn update(&self, gpu: &mut Option<Self::GPU>, device: &wgpu::Device, queue: &wgpu::Queue) {
    gpu.get_or_insert_with(|| device.create_sampler(&convert(*self)));
  }
}

impl BindableResource for wgpu::Sampler {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::Sampler(self)
  }
  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Sampler {
      comparison: false,
      filtering: true,
    }
  }
}

pub type SceneSampler = MaterialBindableItemPair<TextureSampler, wgpu::Sampler>;

impl Scene {
  pub fn add_sampler(&mut self, sampler: TextureSampler) -> SamplerHandle {
    self.samplers.insert(MaterialBindableItemPair::new(sampler))
  }
}

pub fn convert_wrap(mode: AddressMode) -> wgpu::AddressMode {
  match mode {
    AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
    AddressMode::Repeat => wgpu::AddressMode::Repeat,
    AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
    AddressMode::ClampToBorder => wgpu::AddressMode::ClampToBorder,
  }
}
pub fn convert_filter(mode: FilterMode) -> wgpu::FilterMode {
  match mode {
    FilterMode::Nearest => wgpu::FilterMode::Nearest,
    FilterMode::Linear => wgpu::FilterMode::Linear,
  }
}

pub fn convert(sampler: TextureSampler) -> wgpu::SamplerDescriptor<'static> {
  wgpu::SamplerDescriptor {
    label: None,
    address_mode_u: convert_wrap(sampler.address_mode_u),
    address_mode_v: convert_wrap(sampler.address_mode_v),
    address_mode_w: convert_wrap(sampler.address_mode_w),
    mag_filter: convert_filter(sampler.mag_filter),
    min_filter: convert_filter(sampler.min_filter),
    mipmap_filter: convert_filter(sampler.mipmap_filter),
    ..Default::default()
  }
}
