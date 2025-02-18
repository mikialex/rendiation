use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod combined;
pub use combined::*;

pub trait AbstractStorageBuffer<T>: Clone
where
  T: Std430MaybeUnsized + ShaderValueAbstractPtrAccess<Self::ShaderPtr> + ?Sized,
{
  type ShaderPtr: AbstractShaderPtr;
  fn get_gpu_buffer_view(&self) -> &GPUBufferView;
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderAccessorOf<T, Self::ShaderPtr>;
}

impl<T> AbstractStorageBuffer<T> for StorageBufferDataView<T>
where
  T: Std430MaybeUnsized
    + ShaderValueAbstractPtrAccess<ShaderNodeRawHandle>
    + ShaderMaybeUnsizedValueNodeType
    + ?Sized,
{
  type ShaderPtr = ShaderNodeRawHandle;
  fn get_gpu_buffer_view(&self) -> &GPUBufferView {
    &self.view
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderAccessorOf<T, Self::ShaderPtr> {
    T::create_accessor_from_raw_ptr(bind_builder.bind_by(self).handle())
  }
}
