use crate::*;

#[derive(Clone)]
pub struct DeviceBumpAllocationInstance<T: Std430 + ShaderSizedValueNodeType> {
  pub storage: StorageBufferDataView<[T]>,
  pub bump_size: StorageBufferDataView<DeviceAtomic<u32>>,
  pub current_size: StorageBufferDataView<u32>, // todo, merge with bump size
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceBumpAllocationInstance<T> {
  pub fn new(size: usize, device: &GPUDevice) -> Self {
    Self {
      storage: create_gpu_read_write_storage(size * std::mem::size_of::<T>(), device),
      current_size: create_gpu_read_write_storage(StorageBufferInit::WithInit(&0), device),
      bump_size: create_gpu_read_write_storage::<DeviceAtomic<u32>>(
        StorageBufferInit::WithInit(&DeviceAtomic(0)),
        device,
      ),
    }
  }

  pub fn prepare_dispatch_size(
    &self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
    workgroup_size: u32,
  ) -> StorageBufferDataView<DispatchIndirectArgsStorage> {
    let size = device.make_indirect_dispatch_size_buffer();
    let hasher = shader_hasher_from_marker_ty!(SizeCompute);
    let workgroup_size = create_gpu_readonly_storage(&workgroup_size, device);

    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |device| {
      compute_shader_builder()
        .entry(|cx| {
          let input_current_size = cx.bind_by(&self.current_size);
          let output = cx.bind_by(&size);
          let workgroup_size = cx.bind_by(&workgroup_size);

          let size = ENode::<DispatchIndirectArgsStorage> {
            x: device_compute_dispatch_size(input_current_size.load(), workgroup_size.load()),
            y: val(1),
            z: val(1),
          }
          .construct();

          output.store(size);
        })
        .create_compute_pipeline(device)
        .unwrap()
    });

    BindingBuilder::new_as_compute()
      .with_bind(&self.storage)
      .with_bind(&self.current_size)
      .with_bind(&workgroup_size)
      .setup_compute_pass(pass, device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);

    size
  }

  /// because bump allocation or deallocation may overflow or underflow,
  /// so the size commit pass is required for any allocation or deallocation
  pub fn commit_size(
    &self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
    previous_is_allocate: bool,
  ) {
    let hasher = shader_hasher_from_marker_ty!(SizeCommitter).with_hash(previous_is_allocate);

    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |device| {
      compute_shader_builder()
        .config_work_group_size(1)
        .entry(|cx| {
          let bump_size = cx.bind_by(&self.bump_size);
          let current_size = cx.bind_by(&self.current_size);
          let delta = bump_size.atomic_load();
          if previous_is_allocate {
            current_size.store(current_size.load() + delta);
          } else {
            current_size.store(current_size.load() - delta);
          }
          bump_size.atomic_store(val(0));
        })
        .create_compute_pipeline(device)
        .unwrap()
    });

    BindingBuilder::new_as_compute()
      .with_bind(&self.bump_size)
      .with_bind(&self.current_size)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(1, 1, 1);
  }

  pub fn drain_self_into_the_other(
    &self,
    the_other: &Self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
  ) {
    let size = self.prepare_dispatch_size(pass, device, 256);

    let hasher = shader_hasher_from_marker_ty!(Drainer);

    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |device| {
      compute_shader_builder()
        .entry(|cx| {
          let input = cx.bind_by(&self.storage);
          let input_current_size = cx.bind_by(&self.current_size);
          let output = cx.bind_by(&the_other.storage);
          let output_current_size = cx.bind_by(&the_other.current_size);
          let output_offset = output_current_size.load();

          let id = cx.global_invocation_id().x();
          if_by(id.equals(0), || {
            let new_size = output_offset + input_current_size.load();
            output_current_size.store(new_size);
            input_current_size.store(0);
          });

          output
            .index(id + output_offset)
            .store(input.index(id).load());
        })
        .create_compute_pipeline(device)
        .unwrap()
    });

    BindingBuilder::new_as_compute()
      .with_bind(&self.storage)
      .with_bind(&self.current_size)
      .with_bind(&the_other.storage)
      .with_bind(&the_other.current_size)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups_indirect_owned(&size);
  }

  pub fn build_allocator_shader(
    &self,
    cx: &mut ComputeCx,
  ) -> DeviceBumpAllocationInvocationInstance<T> {
    DeviceBumpAllocationInvocationInstance {
      storage: cx.bind_by(&self.storage),
      bump_size: cx.bind_by(&self.bump_size),
      current_size: cx.bind_by(&self.current_size),
    }
  }
  pub fn bind_allocator(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.storage);
    cx.bind(&self.bump_size);
    cx.bind(&self.current_size);
  }

  pub fn build_deallocator_shader(
    &self,
    cx: &mut ComputeCx,
  ) -> DeviceBumpDeAllocationInvocationInstance<T> {
    DeviceBumpDeAllocationInvocationInstance {
      storage: cx.bind_by(&self.storage),
      bump_size: cx.bind_by(&self.bump_size),
      current_size: cx.bind_by(&self.current_size),
    }
  }
}

pub struct DeviceBumpAllocationInvocationInstance<T: Std430> {
  storage: StorageNode<[T]>,
  bump_size: StorageNode<DeviceAtomic<u32>>,
  current_size: StorageNode<u32>,
}

impl<T: Std430 + ShaderNodeType> DeviceBumpAllocationInvocationInstance<T> {
  /// can not use with bump_deallocate in the same dispatch
  pub fn bump_allocate(&self, item: Node<T>) -> (Node<u32>, Node<bool>) {
    let write_idx = self.bump_size.atomic_add(val(1));
    let out_of_bound =
      write_idx.greater_equal_than(self.storage.array_length() - self.current_size.load());
    if_by(out_of_bound.not(), || {
      self.storage.index(write_idx).store(item)
    });
    (write_idx, out_of_bound)
  }
}

pub struct DeviceBumpDeAllocationInvocationInstance<T: Std430> {
  storage: StorageNode<[T]>,
  bump_size: StorageNode<DeviceAtomic<u32>>,
  current_size: StorageNode<u32>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceBumpDeAllocationInvocationInstance<T> {
  /// can not use with bump_allocate in the same dispatch
  pub fn bump_deallocate(&self) -> (Node<T>, Node<bool>) {
    let write_idx = self.bump_size.atomic_add(val(1));
    let out_of_bound = write_idx.greater_equal_than(self.current_size.load());
    let output = zeroed_val().make_local_var();
    if_by(out_of_bound.not(), || {
      output.store(self.storage.index(write_idx).load())
    });
    (output.load(), out_of_bound)
  }
}
