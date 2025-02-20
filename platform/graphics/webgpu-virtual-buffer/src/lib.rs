//! this crate's feature allows user create rw storage buffer from a single buffer pool
//! to workaround the binding limitation on some platform.

#![feature(hash_raw_entry)]

use std::num::NonZeroU64;
use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod combine;
pub(crate) use combine::*;
mod storage;
pub use storage::*;

pub trait AbstractStorageBuffer<T>: Clone
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferView;
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T>;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}

impl<T> AbstractStorageBuffer<T> for StorageBufferDataView<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferView {
    self.view.clone()
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    bind_builder.bind_by(self)
  }
  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(self);
  }
}

pub trait AbstractUniformBuffer<T>: Clone
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferView;
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T>;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}

impl<T> AbstractUniformBuffer<T> for UniformBufferDataView<T>
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferView {
    self.gpu.view.clone()
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    _: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T> {
    bind_builder.bind_by(self)
  }
  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(self);
  }
}
