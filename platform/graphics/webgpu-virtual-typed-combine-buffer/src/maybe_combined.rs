//! these just application level bridging code to dynamically control if combine is enabled
//!
//! add anything wanted here at will

use crate::*;

pub struct StorageBufferCombineGuard {
  outside_allocator: Box<dyn AbstractStorageAllocator>,
}

impl StorageBufferCombineGuard {
  pub fn end(mut self, cx: &mut QueryGPUHookCx) -> StorageBufferCombineRestart {
    std::mem::swap(&mut cx.storage_allocator, &mut self.outside_allocator);
    StorageBufferCombineRestart {
      combined_allocator: self.outside_allocator,
    }
  }
}

pub struct StorageBufferCombineRestart {
  combined_allocator: Box<dyn AbstractStorageAllocator>,
}

impl StorageBufferCombineRestart {
  pub fn restart(mut self, cx: &mut QueryGPUHookCx) -> StorageBufferCombineGuard {
    std::mem::swap(&mut cx.storage_allocator, &mut self.combined_allocator);
    StorageBufferCombineGuard {
      outside_allocator: self.combined_allocator,
    }
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

  pub fn allocate_single(
    &self,
    device: &GPUDevice,
    label: &str,
  ) -> AbstractStorageBuffer<DeviceAtomic<u32>> {
    if let Self::Combined(combined) = self {
      combined.allocate_single(device, label)
    } else {
      DefaultStorageAllocator.allocate(4, device, label)
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

  pub fn allocate_single(
    &self,
    device: &GPUDevice,
    label: &str,
  ) -> AbstractStorageBuffer<DeviceAtomic<T>> {
    self.internal.allocate(4, device, label)
  }

  pub fn allocate_atomic_array(
    &self,
    atomic_count: u32,
    device: &GPUDevice,
    label: &str,
  ) -> AbstractStorageBuffer<[DeviceAtomic<T>]> {
    self
      .internal
      .allocate(4 * atomic_count as u64, device, label)
  }
}
