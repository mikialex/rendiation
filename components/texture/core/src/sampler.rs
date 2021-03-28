use crate::{AddressMode, FilterMode};

pub struct Sampler {
  pub address_mode_u: AddressMode,
  pub address_mode_v: AddressMode,
  pub mag_filter: FilterMode,
  pub min_filter: FilterMode,
  pub mipmap_filter: FilterMode,
}
