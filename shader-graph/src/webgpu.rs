use crate::{ShaderGraphSampler, ShaderGraphTexture, ShaderGraphUBO};
use rendiation_webgpu::*;

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

use rendiation_ral::*;
impl<'a, T: ShaderGraphUBO + 'static> WGPUBindgroupItem<'a> for T {
  type Type = UniformBufferRef<'a, WGPURenderer, T>;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindBuffer(item.data)
  }

  // oh my god we need specialization here in the future
  fn to_layout_type() -> BindingType {
    BindingType::UniformBuffer { dynamic: false }
  }
}

impl<T: RALBackend> RALBindgroupHandle<T> for ShaderGraphTexture {
  type HandleType = TextureViewHandle<T>;
}
impl<'a, T: RALBackend> RALBindgroupItem<'a, T> for ShaderGraphTexture {
  type Resource = &'a T::TextureView;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource {
    resources.texture_views.get(handle).unwrap().resource()
  }
}

impl<T: RALBackend> RALBindgroupHandle<T> for ShaderGraphSampler {
  type HandleType = SamplerHandle<T>;
}
impl<'a, T: RALBackend> RALBindgroupItem<'a, T> for ShaderGraphSampler {
  type Resource = &'a T::Sampler;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource {
    resources.samplers.get(handle).unwrap().resource()
  }
}
