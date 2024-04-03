use crate::*;

mod flat;
pub use flat::*;
mod mr;
pub use mr::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default, Debug, PartialEq)]
pub struct TextureSamplerHandlePair {
  pub texture_handle: u32,
  pub sampler_handle: u32,
}

pub struct TextureSamplerIndirectProvider {
  pub texture2ds: Box<dyn ReactiveCollection<AllocIdx<SceneTexture2dEntity>, u32>>,
  pub samplers: Box<dyn ReactiveCollection<AllocIdx<SceneSamplerEntity>, u32>>,
}
