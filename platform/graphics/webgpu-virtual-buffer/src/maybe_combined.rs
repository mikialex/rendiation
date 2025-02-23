//! these just application level bridging code to dynamically control if combine is enabled
//!
//! add anything wanted here at will

use crate::*;

#[derive(Clone)]
pub enum MaybeCombinedStorageAllocator {
  Combined(CombinedStorageBufferAllocator),
  Default,
}

impl MaybeCombinedStorageAllocator {
  /// label must unique across binding
  pub fn new(label: impl Into<String>, enable_combine: bool, use_packed_layout: bool) -> Self {
    if enable_combine {
      Self::Combined(CombinedStorageBufferAllocator::new(
        label,
        use_packed_layout,
      ))
    } else {
      Self::Default
    }
  }

  pub fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
    &self,
    byte_size: u64,
    device: &GPUDevice,
  ) -> BoxedAbstractStorageBuffer<T> {
    if let Self::Combined(combined) = self {
      Box::new(combined.allocate(byte_size))
    } else {
      Box::new(create_gpu_read_write_storage::<T>(
        StorageBufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap()),
        &device,
      ))
    }
  }

  pub fn rebuild(&self, gpu: &GPU) {
    if let Self::Combined(combined) = self {
      combined.rebuild(gpu);
    }
  }
}

#[derive(Clone)]
pub enum MaybeCombinedAtomicU32StorageAllocator {
  Combined(CombinedAtomicArrayStorageBufferAllocator<u32>),
  Default,
}

impl MaybeCombinedAtomicU32StorageAllocator {
  /// label must unique across binding
  pub fn new(label: impl Into<String>, enable_combine: bool) -> Self {
    if enable_combine {
      Self::Combined(CombinedAtomicArrayStorageBufferAllocator::new(label))
    } else {
      Self::Default
    }
  }

  pub fn allocate_single(
    &self,
    device: &GPUDevice,
  ) -> BoxedAbstractStorageBuffer<DeviceAtomic<u32>> {
    if let Self::Combined(combined) = self {
      Box::new(combined.allocate_single_atomic())
    } else {
      Box::new(create_gpu_read_write_storage::<DeviceAtomic<u32>>(
        StorageBufferInit::Zeroed(NonZeroU64::new(4).unwrap()),
        &device,
      ))
    }
  }

  pub fn rebuild(&self, gpu: &GPU) {
    if let Self::Combined(combined) = self {
      combined.rebuild(gpu);
    }
  }
}
