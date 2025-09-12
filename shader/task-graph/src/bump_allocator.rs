use crate::*;

#[derive(Clone)]
pub struct DeviceBumpAllocationInstance<T: Std430 + ShaderSizedValueNodeType> {
  pub storage: AbstractStorageBuffer<[T]>,
  pub bump_size: AbstractStorageBuffer<DeviceAtomic<u32>>,
  pub current_size: AbstractStorageBuffer<u32>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceBumpAllocationInstance<T> {
  pub fn new(
    size: usize,
    device: &GPUDevice,
    allocator: &dyn AbstractStorageAllocator,
    atomic_allocator: &MaybeCombinedAtomicU32StorageAllocator,
  ) -> Self {
    let storage_byte_size = std::mem::size_of::<T>() * size;
    Self {
      storage: allocator.allocate(storage_byte_size as u64, device, None),
      current_size: allocator.allocate(4, device, None),
      bump_size: atomic_allocator.allocate_single(device),
    }
  }

  pub fn reset(&self, cx: &mut DeviceParallelComputeCtx) {
    cx.record_pass(|pass, device| {
      let hasher = shader_hasher_from_marker_ty!(SizeClear);
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder.config_work_group_size(1);
        let current_size = builder.bind_by(&self.current_size);
        let bump_size = builder.bind_by(&self.bump_size);
        current_size.store(val(0));
        bump_size.atomic_store(val(0));
        builder
      });

      BindingBuilder::default()
        .with_bind(&self.current_size)
        .with_bind(&self.bump_size)
        .setup_compute_pass(pass, device, &pipeline);

      pass.dispatch_workgroups(1, 1, 1);
    });
  }

  pub async fn debug_execution(&self, cx: &mut DeviceParallelComputeCtx<'_>) -> Vec<T> {
    let current_size = self.current_size.get_gpu_buffer_view().unwrap();
    let current_size = StorageBufferDataView::<u32>::try_from_raw(current_size).unwrap();
    let size = cx.read_sized_storage_array(&current_size);

    let storage = self.storage.get_gpu_buffer_view().unwrap();
    let storage = StorageBufferDataView::try_from_raw(storage).unwrap();
    let storage = cx.read_storage_array(&storage);

    cx.submit_recorded_work_and_continue();

    let mut storage = storage.await.unwrap();
    let size = size.await.unwrap();

    storage.truncate(size as usize);
    storage
  }

  pub fn prepare_dispatch_size(
    &self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
    workgroup_size: u32,
  ) -> StorageBufferReadonlyDataView<DispatchIndirectArgsStorage> {
    let size = device.make_indirect_dispatch_size_buffer();
    let hasher = shader_hasher_from_marker_ty!(SizeCompute);
    let workgroup_size = create_gpu_readonly_storage(&workgroup_size, device);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      let input_current_size = builder.bind_by(&self.current_size);
      let output = builder.bind_by(&size);
      let workgroup_size = builder.bind_by(&workgroup_size);

      let size = ENode::<DispatchIndirectArgsStorage> {
        x: device_compute_dispatch_size(input_current_size.load(), workgroup_size.load()),
        y: val(1),
        z: val(1),
      }
      .construct();

      output.store(size);
      builder
    });

    BindingBuilder::default()
      .with_bind(&self.current_size)
      .with_bind(&size)
      .with_bind(&workgroup_size)
      .setup_compute_pass(pass, device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);

    size.into_readonly_view()
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

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);
      let bump_size = builder.bind_by(&self.bump_size);
      let current_size = builder.bind_by(&self.current_size);
      let array = builder.bind_by(&self.storage);

      let delta = bump_size.atomic_load();
      let current_size_load = current_size.load();

      if previous_is_allocate {
        if_by(
          delta.greater_than(array.array_length() - current_size_load),
          || current_size.store(array.array_length()),
        )
        .else_by(|| current_size.store(current_size_load + delta))
      } else {
        if_by(delta.greater_than(current_size_load), || {
          current_size.store(val(0))
        })
        .else_by(|| current_size.store(current_size_load - delta));
      }
      bump_size.atomic_store(val(0));
      builder
    });

    BindingBuilder::default()
      .with_bind(&self.bump_size)
      .with_bind(&self.current_size)
      .with_bind(&self.storage)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(1, 1, 1);
  }

  /// return drained size
  ///
  /// self and the other must be committed size
  pub fn drain_self_into_the_other(
    &self,
    the_other: &Self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
  ) -> StorageBufferReadonlyDataView<DispatchIndirectArgsStorage> {
    let size = self.prepare_dispatch_size(pass, device, 256);

    let hasher = shader_hasher_from_marker_ty!(Drainer);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      let input = builder.bind_by(&self.storage);
      let input_current_size = builder.bind_by(&self.current_size);
      let output = builder.bind_by(&the_other.storage);
      let output_current_size = builder.bind_by(&the_other.current_size);
      let output_offset = output_current_size.load();

      let id = builder.global_invocation_id().x();

      if_by(id.less_than(input_current_size.load()), || {
        output
          .index(id + output_offset)
          .store(input.index(id).load());
      });

      builder
    });

    BindingBuilder::default()
      .with_bind(&self.storage)
      .with_bind(&self.current_size)
      .with_bind(&the_other.storage)
      .with_bind(&the_other.current_size)
      .setup_compute_pass(pass, device, &pipeline);
    pass.dispatch_workgroups_indirect_by_buffer_resource_view(&size);

    let hasher = shader_hasher_from_marker_ty!(DrainerSizeSet);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);
      let input_current_size = builder.bind_by(&self.current_size);
      let output_current_size = builder.bind_by(&the_other.current_size);
      let output_offset = output_current_size.load();

      let id = builder.global_invocation_id().x();
      if_by(id.equals(0), || {
        let new_size = output_offset + input_current_size.load();
        output_current_size.store(new_size);
        input_current_size.store(val(0));
      });

      builder
    });

    BindingBuilder::default()
      .with_bind(&self.current_size)
      .with_bind(&the_other.current_size)
      .setup_compute_pass(pass, device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);

    size
  }

  pub fn build_allocator_shader(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
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
    cx: &mut ShaderComputePipelineBuilder,
  ) -> DeviceBumpDeAllocationInvocationInstance<T> {
    DeviceBumpDeAllocationInvocationInstance {
      storage: cx.bind_by(&self.storage),
      bump_size: cx.bind_by(&self.bump_size),
      current_size: cx.bind_by(&self.current_size),
    }
  }
}

#[derive(Clone)]
pub struct DeviceBumpAllocationInvocationInstance<T: Std430> {
  pub storage: ShaderPtrOf<[T]>,
  pub bump_size: ShaderPtrOf<DeviceAtomic<u32>>,
  pub current_size: ShaderPtrOf<u32>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceBumpAllocationInvocationInstance<T> {
  /// can not use with bump_allocate in the same dispatch
  ///
  /// return if success
  #[must_use]
  pub fn bump_allocate_counts(&self, count: Node<u32>) -> (Node<u32>, Node<bool>) {
    let bumped = self.bump_size.atomic_add(count);
    let current_size = self.current_size.load();
    let in_bound = bumped.less_equal_than(self.storage.array_length() - current_size);
    let write_idx = bumped + current_size;
    (write_idx, in_bound)
  }

  /// can not use with bump_deallocate in the same dispatch
  ///
  /// return if success
  #[must_use]
  pub fn bump_allocate_by(
    &self,
    count: Node<u32>,
    on_success_access: impl FnOnce(ShaderPtrOf<[T]>, Node<u32>),
  ) -> (Node<u32>, Node<bool>) {
    let (write_idx, in_bound) = self.bump_allocate_counts(count);
    if_by(in_bound, || {
      on_success_access(self.storage.clone(), write_idx);
    });
    (write_idx, in_bound)
  }

  /// can not use with bump_deallocate in the same dispatch
  ///
  /// return if success
  #[must_use]
  pub fn bump_allocate(&self, item: Node<T>) -> (Node<u32>, Node<bool>) {
    self.bump_allocate_by(val(1), |storage, write_idx| {
      storage.index(write_idx).store(item);
    })
  }
}

#[derive(Clone)]
pub struct DeviceBumpDeAllocationInvocationInstance<T: Std430> {
  pub storage: ShaderPtrOf<[T]>,
  pub bump_size: ShaderPtrOf<DeviceAtomic<u32>>,
  pub current_size: ShaderPtrOf<u32>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceBumpDeAllocationInvocationInstance<T> {
  /// can not use with bump_allocate in the same dispatch
  ///
  /// return if success
  #[must_use]
  pub fn bump_deallocate_counts(&self, count: Node<u32>) -> (Node<u32>, Node<bool>) {
    let bumped = self.bump_size.atomic_add(count);
    let current_size = self.current_size.load();
    let in_bound = bumped.less_equal_than(current_size);
    let read_idx = current_size - bumped - count;
    (read_idx, in_bound)
  }

  /// can not use with bump_allocate in the same dispatch
  ///
  /// return if success
  #[must_use]
  pub fn bump_deallocate(&self) -> (Node<T>, Node<bool>) {
    let (read_idx, in_bound) = self.bump_deallocate_counts(val(1));

    let output = zeroed_val::<T>().make_local_var();
    if_by(in_bound, || {
      output.store(self.storage.index(read_idx).load())
    });
    (output.load(), in_bound)
  }
}
