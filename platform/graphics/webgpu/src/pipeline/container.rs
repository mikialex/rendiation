pub use crate::*;

impl ShaderBindingProvider for RenderTargetView {
  type Node = ShaderBinding<ShaderTexture2D>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }
}

impl<T> ShaderBindingProvider for UniformBufferCachedDataView<T>
where
  T: ShaderSizedValueNodeType + Std140 + SizedShaderAbstractPtrAccess,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderReadonlyPtrOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_readonly_view_from_raw_ptr(Box::new(node.handle()))
  }
}

impl<T> ShaderBindingProvider for UniformBufferDataView<T>
where
  T: ShaderSizedValueNodeType + Std140 + SizedShaderAbstractPtrAccess,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderReadonlyPtrOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_readonly_view_from_raw_ptr(Box::new(node.handle()))
  }
}

impl<T> ShaderBindingProvider for StorageBufferReadonlyDataView<T>
where
  T: ShaderMaybeUnsizedValueNodeType + Std430MaybeUnsized + ShaderAbstractPtrAccess + ?Sized,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderReadonlyPtrOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_readonly_view_from_raw_ptr(Box::new(node.handle()))
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
  T: ShaderMaybeUnsizedValueNodeType + Std430MaybeUnsized + ShaderAbstractPtrAccess + ?Sized,
{
  type Node = ShaderBinding<T>;
  type ShaderInstance = ShaderPtrOf<T>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    T::create_view_from_raw_ptr(Box::new(node.handle()))
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      writeable_if_storage: true,
      ty: Self::Node::ty(),
    }
  }
}

impl<D, F> ShaderBindingProvider for GPUTypedTextureView<D, F>
where
  D: ShaderTextureDimension,
  F: ShaderTextureKind,
{
  type Node = ShaderBinding<ShaderTexture<D, F>>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
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

map_shader_ty!(GPUSamplerView, ShaderSampler);
map_shader_ty!(GPUComparisonSamplerView, ShaderCompareSampler);
map_shader_ty!(GPUTlasView, ShaderAccelerationStructure);
