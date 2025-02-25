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

  pub fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
  ) -> BoxedAbstractStorageBufferDynTyped {
    if let Self::Combined(combined) = self {
      Box::new(combined.allocate_dyn(byte_size, ty_desc))
    } else {
      #[derive(Clone)]
      struct DynTypedStorageBuffer {
        buffer: GPUBufferResourceView,
        ty: MaybeUnsizedValueType,
      }
      impl AbstractStorageBufferDynTyped for DynTypedStorageBuffer {
        fn get_gpu_buffer_view(&self) -> GPUBufferResourceView {
          self.buffer.clone()
        }

        fn bind_shader(
          &self,
          bind_builder: &mut ShaderBindGroupBuilder,
          _: &mut SemanticRegistry,
        ) -> BoxedShaderPtr {
          let ty = self.ty.clone().into_shader_single_ty();
          let desc = ShaderBindingDescriptor {
            should_as_storage_buffer_if_is_buffer_like: true,
            ty: ShaderValueType::Single(ty),
            writeable_if_storage: true,
          };
          let node = bind_builder.binding_dyn(desc).using();
          Box::new(node)
        }

        fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
          bind_builder.bind_dyn(self.buffer.get_binding_build_source());
        }
      }

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
