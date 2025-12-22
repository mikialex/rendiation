//! these just application level bridging code to dynamically control if combine is enabled
//!
//! add anything wanted here at will

use crate::*;

pub struct StorageBufferCombineGuard {
  outside_allocator: Box<dyn AbstractStorageAllocator>,
}

impl StorageBufferCombineGuard {
  pub fn end(self, cx: &mut QueryGPUHookCx) {
    cx.storage_allocator = self.outside_allocator;
  }
}

/// enable config only take effect in first pass
pub fn use_readonly_storage_buffer_combine(
  cx: &mut QueryGPUHookCx,
  label: impl Into<String>,
  enable_combine: bool,
) -> StorageBufferCombineGuard {
  let outside_allocator = cx.storage_allocator.clone();

  let (cx, allocator) = cx.use_gpu_init(|gpu, _| {
    create_maybe_combined_storage_allocator(
      gpu,
      label,
      enable_combine,
      false,
      true,
      outside_allocator.clone(),
    )
  });

  cx.storage_allocator = allocator.clone();

  StorageBufferCombineGuard { outside_allocator }
}

/// enable config only take effect in first pass
pub fn use_scoped_readonly_storage_buffer_combine(
  cx: &mut QueryGPUHookCx,
  label: impl Into<String>,
  enable_combine: bool,
  scope: impl FnOnce(&mut QueryGPUHookCx),
) {
  let g = use_readonly_storage_buffer_combine(cx, label, enable_combine);
  scope(cx);
  g.end(cx);
}

pub fn create_maybe_combined_storage_allocator(
  gpu: &GPU,
  label: impl Into<String>,
  enable_combine: bool,
  use_packed_layout: bool,
  readonly: bool,
  outside_allocator: Box<dyn AbstractStorageAllocator>,
) -> Box<dyn AbstractStorageAllocator> {
  if enable_combine {
    Box::new(CombinedStorageBufferAllocator::new(
      gpu,
      label,
      use_packed_layout,
      readonly,
      outside_allocator,
    ))
  } else {
    outside_allocator
  }
}

pub enum MaybeCombinedAtomicU32StorageAllocator {
  Combined(CombinedAtomicArrayStorageBufferAllocator<u32>),
  Default,
}

impl MaybeCombinedAtomicU32StorageAllocator {
  pub fn new(gpu: &GPU, label: impl Into<String>, enable_combine: bool) -> Self {
    if enable_combine {
      Self::Combined(CombinedAtomicArrayStorageBufferAllocator::new(gpu, label))
    } else {
      Self::Default
    }
  }

  pub fn allocate_single(&self, device: &GPUDevice) -> AbstractStorageBuffer<DeviceAtomic<u32>> {
    if let Self::Combined(combined) = self {
      combined.allocate_single(device)
    } else {
      DefaultStorageAllocator.allocate(4, device, None)
    }
  }
}

pub struct CombinedAtomicArrayStorageBufferAllocator<T> {
  atomic_ty: PhantomData<T>,
  internal: Box<dyn AbstractStorageAllocator>,
}

impl<T: AtomicityShaderNodeType> CombinedAtomicArrayStorageBufferAllocator<T> {
  pub fn new(gpu: &GPU, label: impl Into<String>) -> Self {
    Self {
      atomic_ty: PhantomData,
      internal: Box::new(CombinedStorageBufferAllocator::new_atomic::<T>(gpu, label)),
    }
  }

  pub fn allocate_single(&self, device: &GPUDevice) -> AbstractStorageBuffer<DeviceAtomic<T>> {
    self.internal.allocate(4, device, None)
  }

  pub fn allocate_atomic_array(
    &self,
    atomic_count: u32,
    device: &GPUDevice,
  ) -> AbstractStorageBuffer<[DeviceAtomic<T>]> {
    self
      .internal
      .allocate(4 * atomic_count as u64, device, None)
  }
}
