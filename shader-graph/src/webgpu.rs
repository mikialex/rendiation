use crate::{ShaderGraphSampler, ShaderGraphTexture, ShaderGraphUBO};
use rendiation_webgpu::*;
use std::ops::Range;

pub trait WGPUBindgroupItem<'a> {
  type Type;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a>;
  fn to_layout_type() -> BindingType;
}

impl<'a> WGPUBindgroupItem<'a> for ShaderGraphTexture {
  type Type = &'a TextureView;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindTexture(item)
  }
  fn to_layout_type() -> BindingType {
    BindingType::SampledTexture {
      multisampled: false,
      component_type: wgpu::TextureComponentType::Float,
      dimension: wgpu::TextureViewDimension::D2,
    }
  }
}

impl<'a> WGPUBindgroupItem<'a> for ShaderGraphSampler {
  type Type = &'a WGPUSampler;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindSampler(item)
  }
  // any other situation could be inject by generics over ShaderGraphSampler
  fn to_layout_type() -> BindingType {
    BindingType::Sampler { comparison: false }
  }
}

// oh my god we need specialization here in the future
impl<'a, T: ShaderGraphUBO> WGPUBindgroupItem<'a> for T {
  type Type = (&'a WGPUBuffer, Range<u64>);
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindBuffer(item)
  }
  fn to_layout_type() -> BindingType {
    BindingType::UniformBuffer { dynamic: false }
  }
}
