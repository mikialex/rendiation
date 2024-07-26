use crate::*;

pub struct DeviceBumpAllocationInstance<T: Std430 + ShaderSizedValueNodeType> {
  storage: StorageBufferDataView<[T]>,
  bump_size: StorageBufferDataView<DeviceAtomic<u32>>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceBumpAllocationInstance<T> {
  pub fn new(size: usize, device: &GPUDevice) -> Self {
    Self {
      storage: create_gpu_read_write_storage(size * std::mem::size_of::<T>(), device),
      bump_size: create_gpu_read_write_storage::<DeviceAtomic<u32>>(
        StorageBufferInit::WithInit(&DeviceAtomic(0)),
        device,
      ),
    }
  }

  pub fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> DeviceBumpAllocationInvocationInstance<T> {
    DeviceBumpAllocationInvocationInstance {
      storage: builder.entry_by(|cx| cx.bind_by(&self.storage)),
      bump_size: builder.entry_by(|cx| cx.bind_by(&self.bump_size)),
    }
  }
}

pub struct DeviceBumpAllocationInvocationInstance<T: Std430> {
  storage: StorageNode<[T]>,
  bump_size: StorageNode<DeviceAtomic<u32>>,
}

impl<T: Std430 + ShaderNodeType> DeviceBumpAllocationInvocationInstance<T> {
  pub fn allocate(&self, item: Node<T>) -> (Node<u32>, Node<bool>) {
    let write_idx = self.bump_size.atomic_add(val(1));
    let out_of_bound = write_idx.greater_equal_than(self.storage.array_length());
    if_by(out_of_bound.not(), || {
      self.storage.index(write_idx).store(item)
    });
    (write_idx, out_of_bound)
  }
}

pub struct DeviceUntypedBumpAllocationInstance {
  storage: GPUBufferResourceView,
  ty_desc: ShaderSizedValueType,
  bump_size: StorageBufferDataView<DeviceAtomic<u32>>,
}

pub struct DeviceUntypedBumpAllocationInvocationInstance {
  storage: NodeUntyped,
  bump_size: StorageNode<DeviceAtomic<u32>>,
}

impl DeviceUntypedBumpAllocationInvocationInstance {
  pub fn allocate(&self, item: NodeUntyped) -> (Node<u32>, Node<bool>) {
    todo!()
  }
}
