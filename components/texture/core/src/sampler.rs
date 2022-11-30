use crate::{AddressMode, FilterMode};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextureSampler {
  pub address_mode_u: AddressMode,
  pub address_mode_v: AddressMode,
  pub address_mode_w: AddressMode,
  pub mag_filter: FilterMode,
  pub min_filter: FilterMode,
  pub mipmap_filter: FilterMode,
}

impl TextureSampler {
  pub fn tri_linear_repeat() -> Self {
    Self {
      address_mode_u: AddressMode::Repeat,
      address_mode_v: AddressMode::Repeat,
      address_mode_w: AddressMode::Repeat,
      mag_filter: FilterMode::Linear,
      min_filter: FilterMode::Linear,
      mipmap_filter: FilterMode::Linear,
    }
  }
}

impl Default for TextureSampler {
  fn default() -> Self {
    Self {
      address_mode_u: AddressMode::ClampToEdge,
      address_mode_v: AddressMode::ClampToEdge,
      address_mode_w: AddressMode::ClampToEdge,
      mag_filter: FilterMode::Nearest,
      min_filter: FilterMode::Nearest,
      mipmap_filter: FilterMode::Nearest,
    }
  }
}

#[cfg(feature = "webgpu")]
impl<'a> From<TextureSampler> for rendiation_webgpu::SamplerDescriptor<'a> {
  fn from(val: TextureSampler) -> Self {
    fn convert_wrap(mode: AddressMode) -> rendiation_webgpu::AddressMode {
      match mode {
        AddressMode::ClampToEdge => rendiation_webgpu::AddressMode::ClampToEdge,
        AddressMode::Repeat => rendiation_webgpu::AddressMode::Repeat,
        AddressMode::MirrorRepeat => rendiation_webgpu::AddressMode::MirrorRepeat,
      }
    }
    fn convert_filter(mode: FilterMode) -> rendiation_webgpu::FilterMode {
      match mode {
        FilterMode::Nearest => rendiation_webgpu::FilterMode::Nearest,
        FilterMode::Linear => rendiation_webgpu::FilterMode::Linear,
      }
    }

    rendiation_webgpu::SamplerDescriptor {
      label: None,
      address_mode_u: convert_wrap(val.address_mode_u),
      address_mode_v: convert_wrap(val.address_mode_v),
      address_mode_w: convert_wrap(val.address_mode_w),
      mag_filter: convert_filter(val.mag_filter),
      min_filter: convert_filter(val.min_filter),
      mipmap_filter: convert_filter(val.mipmap_filter),
      ..Default::default()
    }
  }
}
