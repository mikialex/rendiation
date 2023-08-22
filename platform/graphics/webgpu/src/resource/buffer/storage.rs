use __core::{marker::PhantomData, num::NonZeroU64};
use rendiation_shader_api::{Std430, Std430MaybeUnsized};

use crate::*;

#[derive(Clone)]
pub struct StorageBufferReadOnlyDataView<T: Std430MaybeUnsized + ?Sized> {
  gpu: GPUBufferResourceView,
  phantom: PhantomData<T>,
}

impl<T: Std430MaybeUnsized + ?Sized> BindableResourceProvider for StorageBufferReadOnlyDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> CacheAbleBindingSource for StorageBufferReadOnlyDataView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_binding_build_source()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> BindableResourceView for StorageBufferReadOnlyDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

impl<T: Std430MaybeUnsized + ?Sized> StorageBufferReadOnlyDataView<T> {
  pub fn create(device: &GPUDevice, data: &T) -> Self {
    let usage = gpu::BufferUsages::STORAGE | gpu::BufferUsages::COPY_DST;
    let bytes = data.bytes();
    let gpu = GPUBuffer::create(device, BufferInit::WithInit(bytes), usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, usage).create_default_view();

    Self {
      gpu,
      phantom: PhantomData,
    }
  }
}

/// just short convenient method
pub fn create_gpu_readonly_storage<T: Std430MaybeUnsized + ?Sized>(
  data: &T,
  device: impl AsRef<GPUDevice>,
) -> StorageBufferReadOnlyDataView<T> {
  StorageBufferReadOnlyDataView::create(device.as_ref(), data)
}

#[derive(Clone)]
pub struct StorageBufferDataView<T: Std430MaybeUnsized + ?Sized> {
  gpu: GPUBufferResourceView,
  phantom: PhantomData<T>,
}

impl<T: Std430MaybeUnsized + ?Sized> BindableResourceProvider for StorageBufferDataView<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.gpu.get_bindable()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> CacheAbleBindingSource for StorageBufferDataView<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.gpu.get_binding_build_source()
  }
}
impl<T: Std430MaybeUnsized + ?Sized> BindableResourceView for StorageBufferDataView<T> {
  fn as_bindable(&self) -> gpu::BindingResource {
    self.gpu.as_bindable()
  }
}

impl<'a, T: Std430> From<&'a [T]> for StorageBufferInit<'a, [T]> {
  fn from(value: &'a [T]) -> Self {
    StorageBufferInit::WithInit(value)
  }
}

impl<'a, T: Std430> From<usize> for StorageBufferInit<'a, [T]> {
  fn from(len: usize) -> Self {
    let byte_len = std::mem::size_of::<T>() * len;
    StorageBufferInit::Zeroed(NonZeroU64::new(byte_len as u64).unwrap())
  }
}

/// just short convenient method
pub fn create_gpu_read_write_storage<'a, T: Std430MaybeUnsized + ?Sized + 'static>(
  data: impl Into<StorageBufferInit<'a, T>>,
  device: impl AsRef<GPUDevice>,
) -> StorageBufferDataView<T> {
  StorageBufferDataView::create(device.as_ref(), data.into())
}

pub enum StorageBufferInit<'a, T: Std430MaybeUnsized + ?Sized> {
  WithInit(&'a T),
  Zeroed(std::num::NonZeroU64),
}

impl<'a, T: Std430MaybeUnsized + ?Sized> StorageBufferInit<'a, T> {
  fn into_buffer_init(self) -> BufferInit<'a> {
    match self {
      StorageBufferInit::WithInit(data) => BufferInit::WithInit(data.bytes()),
      StorageBufferInit::Zeroed(size) => BufferInit::Zeroed(size),
    }
  }
}

impl<T: Std430MaybeUnsized + ?Sized> StorageBufferDataView<T> {
  pub fn create(device: &GPUDevice, data: StorageBufferInit<T>) -> Self {
    let usage = gpu::BufferUsages::STORAGE | gpu::BufferUsages::COPY_DST;
    let gpu = GPUBuffer::create(device, data.into_buffer_init(), usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, usage).create_default_view();

    Self {
      gpu,
      phantom: PhantomData,
    }
  }
}
