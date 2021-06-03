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
