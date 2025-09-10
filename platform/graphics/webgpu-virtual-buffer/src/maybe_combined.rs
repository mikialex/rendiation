//! these just application level bridging code to dynamically control if combine is enabled
//!
//! add anything wanted here at will

use crate::*;

/// enable config only take effect in first pass
pub fn use_storage_buffer_combine(
  cx: &mut QueryGPUHookCx,
  label: impl Into<String>,
  enable: bool,
  scope: impl FnOnce(&mut QueryGPUHookCx, &MaybeCombinedStorageAllocator),
) {
  let (cx, allocator) =
    cx.use_gpu_init(|gpu| MaybeCombinedStorageAllocator::new(gpu, label, enable, false));
  scope(cx, allocator);
}

#[derive(Clone)]
pub enum MaybeCombinedStorageAllocator {
  Combined(CombinedStorageBufferAllocator),
  Default,
}

impl AbstractStorageAllocator for MaybeCombinedStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedAbstractBufferDynTyped {
    if let Self::Combined(combined) = self {
      Box::new(combined.allocate_dyn(byte_size, ty_desc))
    } else {
      // this ty mark is useless actually
      let buffer = create_gpu_read_write_storage::<[u32]>(
        StorageBufferInit::Zeroed(NonZeroU64::new(byte_size).unwrap()),
        &device,
      )
      .gpu;
      let buffer = DynTypedStorageBuffer {
        buffer,
        ty: ty_desc,
      };

      Box::new(buffer)
    }
  }
}

impl MaybeCombinedStorageAllocator {
  /// label must unique across binding
  pub fn new(
    gpu: &GPU,
    label: impl Into<String>,
    enable_combine: bool,
    use_packed_layout: bool,
  ) -> Self {
    if enable_combine {
      Self::Combined(CombinedStorageBufferAllocator::new(
        gpu,
        label,
        use_packed_layout,
      ))
    } else {
      Self::Default
    }
  }
}

pub enum MaybeCombinedAtomicU32StorageAllocator {
  Combined(CombinedAtomicArrayStorageBufferAllocator<u32>),
  Default,
}

impl MaybeCombinedAtomicU32StorageAllocator {
  /// label must unique across binding
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
      DefaultStorageAllocator.allocate(4, device)
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
    self.internal.allocate(4, device)
  }

  pub fn allocate_atomic_array(
    &self,
    atomic_count: u32,
    device: &GPUDevice,
  ) -> AbstractStorageBuffer<[DeviceAtomic<T>]> {
    self.internal.allocate(4 * atomic_count as u64, device)
  }
}
