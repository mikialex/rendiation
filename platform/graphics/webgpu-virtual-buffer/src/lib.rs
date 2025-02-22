//! this crate's feature allows user create rw storage buffer from a single buffer pool
//! to workaround the binding limitation on some platform.

#![feature(hash_raw_entry)]

use std::num::NonZeroU64;
use std::{marker::PhantomData, sync::Arc};

use dyn_clone::DynClone;
use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod combine;
pub(crate) use combine::*;
mod storage;
pub use storage::*;
mod uniform;
pub use uniform::*;
mod maybe_combined;
pub use maybe_combined::*;

pub type BoxedAbstractStorageBuffer<T> = Box<dyn AbstractStorageBuffer<T>>;
pub trait AbstractStorageBuffer<T>: DynClone
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView;
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T>;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}
impl<T> Clone for BoxedAbstractStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}
impl<T> AbstractStorageBuffer<T> for BoxedAbstractStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    (**self).get_gpu_buffer_view()
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderPtrOf<T> {
    (**self).bind_shader(bind_builder, registry)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    (**self).bind_pass(bind_builder)
  }
}

impl<T> AbstractStorageBuffer<T> for StorageBufferDataView<T>
where
  T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    self.resource.create_default_view()
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

pub type BoxedAbstractUniformBuffer<T> = Box<dyn AbstractUniformBuffer<T>>;
pub trait AbstractUniformBuffer<T>: DynClone
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView;
  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T>;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
}
impl<T> Clone for BoxedAbstractUniformBuffer<T> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}
impl<T> AbstractUniformBuffer<T> for BoxedAbstractUniformBuffer<T>
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    (**self).get_gpu_buffer_view()
  }

  fn bind_shader(
    &self,
    bind_builder: &mut ShaderBindGroupBuilder,
    registry: &mut SemanticRegistry,
  ) -> ShaderReadonlyPtrOf<T> {
    (**self).bind_shader(bind_builder, registry)
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    (**self).bind_pass(bind_builder)
  }
}

impl<T> AbstractUniformBuffer<T> for UniformBufferDataView<T>
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
    self.gpu.clone()
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

pub trait ComputeShaderBuilderAbstractBufferExt {
  fn bind_abstract_storage<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    buffer: &impl AbstractStorageBuffer<T>,
  ) -> ShaderPtrOf<T>;
  fn bind_abstract_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    buffer: &impl AbstractUniformBuffer<T>,
  ) -> ShaderReadonlyPtrOf<T>;
}
impl ComputeShaderBuilderAbstractBufferExt for ShaderComputePipelineBuilder {
  fn bind_abstract_storage<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &mut self,
    buffer: &impl AbstractStorageBuffer<T>,
  ) -> ShaderPtrOf<T> {
    buffer.bind_shader(&mut self.bindgroups, &mut self.registry)
  }

  fn bind_abstract_uniform<T>(
    &mut self,
    buffer: &impl AbstractUniformBuffer<T>,
  ) -> ShaderReadonlyPtrOf<T>
  where
    T: ShaderSizedValueNodeType + Std140,
  {
    buffer.bind_shader(&mut self.bindgroups, &mut self.registry)
  }
}
pub trait BindBuilderAbstractBufferExt: Sized {
  fn bind_abstract_storage<T>(&mut self, buffer: &impl AbstractStorageBuffer<T>) -> &mut Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized;
  fn with_bind_abstract_storage<T>(mut self, buffer: &impl AbstractStorageBuffer<T>) -> Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
  {
    self.bind_abstract_storage(buffer);
    self
  }
  fn bind_abstract_uniform<T>(&mut self, buffer: &impl AbstractUniformBuffer<T>) -> &mut Self
  where
    T: ShaderSizedValueNodeType + Std140;
  fn with_bind_abstract_uniform<T>(mut self, buffer: &impl AbstractUniformBuffer<T>) -> Self
  where
    T: ShaderSizedValueNodeType + Std140,
  {
    self.bind_abstract_uniform(buffer);
    self
  }
}
impl BindBuilderAbstractBufferExt for BindingBuilder {
  fn bind_abstract_storage<T>(&mut self, buffer: &impl AbstractStorageBuffer<T>) -> &mut Self
  where
    T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized,
  {
    buffer.bind_pass(self);
    self
  }

  fn bind_abstract_uniform<T: ShaderSizedValueNodeType + Std140>(
    &mut self,
    buffer: &impl AbstractUniformBuffer<T>,
  ) -> &mut Self {
    buffer.bind_pass(self);
    self
  }
}
