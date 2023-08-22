use rendiation_shader_api::*;

pub use crate::*;

impl<T: ShaderSizedValueNodeType + Std140> ShaderBindingProvider for UniformBufferDataView<T> {
  const SPACE: AddressSpace = AddressSpace::Uniform;
  type Node = T;
}

impl<T> ShaderBindingProvider for StorageBufferReadOnlyDataView<T>
where
  T: ShaderMaybeUnsizedValueNodeType + Std430MaybeUnsized + ?Sized,
{
  const SPACE: AddressSpace = AddressSpace::Storage { writeable: false };
  type Node = T;

  fn binding_desc() -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: Self::Node::TYPE,
    }
  }
}

impl<T> ShaderBindingProvider for StorageBufferDataView<T>
where
  T: ShaderMaybeUnsizedValueNodeType + Std430MaybeUnsized + ?Sized,
{
  const SPACE: AddressSpace = AddressSpace::Storage { writeable: true };
  type Node = T;

  fn binding_desc() -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: Self::Node::TYPE,
    }
  }
}

macro_rules! map_shader_ty {
  ($ty: ty, $shader_ty: ty) => {
    impl ShaderBindingProvider for $ty {
      const SPACE: AddressSpace = AddressSpace::Handle;
      type Node = $shader_ty;
    }
  };
}
map_shader_ty!(GPU1DTextureView, ShaderTexture1D);

map_shader_ty!(GPU2DTextureView, ShaderTexture2D);
map_shader_ty!(GPU2DArrayTextureView, ShaderTexture2DArray);

map_shader_ty!(GPUCubeTextureView, ShaderTextureCube);
map_shader_ty!(GPUCubeArrayTextureView, ShaderTextureCubeArray);

map_shader_ty!(GPU3DTextureView, ShaderTexture3D);

map_shader_ty!(GPU2DDepthTextureView, ShaderDepthTexture2D);
map_shader_ty!(GPU2DArrayDepthTextureView, ShaderDepthTexture2DArray);
map_shader_ty!(GPUCubeDepthTextureView, ShaderDepthTextureCube);
map_shader_ty!(GPUCubeArrayDepthTextureView, ShaderDepthTextureCubeArray);

map_shader_ty!(GPUSamplerView, ShaderSampler);
map_shader_ty!(GPUComparisonSamplerView, ShaderCompareSampler);
