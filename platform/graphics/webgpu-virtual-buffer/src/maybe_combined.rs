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
  fn allocate<T: Std430MaybeUnsized + ShaderMaybeUnsizedValueNodeType + ?Sized>(
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

  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedAbstractStorageBufferDynTyped {
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

  pub fn rebuild(&self) {
    if let Self::Combined(combined) = self {
      combined.rebuild();
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

  pub fn rebuild(&self) {
    if let Self::Combined(combined) = self {
      combined.rebuild();
    }
  }
}
