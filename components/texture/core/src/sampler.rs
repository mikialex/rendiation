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
