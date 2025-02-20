use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod combined;
pub use combined::*;

pub trait AbstractStorageBuffer<T>: Clone
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> &GPUBufferView;
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T>;
}

impl<T> AbstractStorageBuffer<T> for StorageBufferDataView<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> &GPUBufferView {
    &self.view
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    bind_builder.bind_by(self)
  }
}
