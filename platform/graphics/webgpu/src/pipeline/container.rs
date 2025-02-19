pub use crate::*;

impl<T> ShaderBindingProvider for UniformBufferCachedDataView<T>
where
  T: ShaderSizedValueNodeType + Std140 + SizedShaderValueAbstractPtrAccess,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderReadonlyAccessorOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_readonly_accessor_from_raw_ptr(Box::new(node.handle()))
  }
}

impl<T> ShaderBindingProvider for UniformBufferDataView<T>
where
  T: ShaderSizedValueNodeType + Std140 + SizedShaderValueAbstractPtrAccess,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderReadonlyAccessorOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_readonly_accessor_from_raw_ptr(Box::new(node.handle()))
  }
}

impl<T> ShaderBindingProvider for StorageBufferReadonlyDataView<T>
where
  T: ShaderMaybeUnsizedValueNodeType + Std430MaybeUnsized + ShaderValueAbstractPtrAccess + ?Sized,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderReadonlyAccessorOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_readonly_accessor_from_raw_ptr(Box::new(node.handle()))
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      writeable_if_storage: false,
      ty: Self::Node::ty(),
    }
  }
}

impl<T> ShaderBindingProvider for StorageBufferDataView<T>
where
  T: ShaderMaybeUnsizedValueNodeType + Std430MaybeUnsized + ShaderValueAbstractPtrAccess + ?Sized,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderAccessorOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_accessor_from_raw_ptr(Box::new(node.handle()))
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      writeable_if_storage: true,
      ty: Self::Node::ty(),
    }
  }
}

macro_rules! map_shader_ty {
  ($ty: ty, $shader_ty: ty) => {
    impl ShaderBindingProvider for $ty {
      type Node = ShaderBinding<$shader_ty>;
      fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
        node
      }
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

map_shader_ty!(GPUMultiSample2DTextureView, ShaderMultiSampleTexture2D);
map_shader_ty!(
  GPUMultiSample2DDepthTextureView,
  ShaderMultiSampleDepthTexture2D
);
