use rendiation_texture::{AddressMode, FilterMode, TextureSampler};

use super::{BindableResource, MaterialHandle, ResourcePair, SamplerHandle, Scene};

pub struct SceneSampler {
  sampler: TextureSampler,
  gpu: Option<wgpu::Sampler>,
  used_by: Vec<MaterialHandle>,
}

impl ResourcePair for SceneSampler {
  type Data = TextureSampler;
  type Resource = Option<wgpu::Sampler>;
  fn data(&self) -> &Self::Data {
    &self.sampler
  }

  fn resource(&self) -> &Self::Resource {
    &self.gpu
  }
}

impl SceneSampler {
  pub fn update(&mut self, device: &wgpu::Device) {
    self
      .gpu
      .get_or_insert_with(|| device.create_sampler(&convert(self.sampler)));
  }

  fn get_gpu(&self) -> &wgpu::Sampler {
    self.gpu.as_ref().unwrap()
  }

  pub fn as_material_bind(&mut self, material: MaterialHandle) -> wgpu::BindingResource {
    self.used_by.push(material);
    self.get_gpu().as_bindable()
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

impl Scene {
  pub fn add_sampler(&mut self, sampler: TextureSampler) -> SamplerHandle {
    self.samplers.insert(SceneSampler {
      sampler,
      gpu: None,
      used_by: Vec::new(),
    })
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
