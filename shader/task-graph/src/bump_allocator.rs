use std::{any::TypeId, hash::Hash};

use crate::*;

pub struct DeviceBumpAllocationInstance<T: Std430 + ShaderSizedValueNodeType> {
  pub storage: StorageBufferDataView<[T]>,
  bump_size: StorageBufferDataView<DeviceAtomic<u32>>,
  current_size: StorageBufferDataView<u32>, // todo, merge with bump size
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

  /// because bump allocation or deallocation may overflow or underflow,
  /// so the size commit pass is required for any allocation or deallocation
  pub fn commit_size(
    &self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
    previous_is_allocate: bool,
  ) {
    struct SizeCommitter;
    let mut hasher = PipelineHasher::default();
    TypeId::of::<SizeCommitter>().hash(&mut hasher);
    previous_is_allocate.hash(&mut hasher);

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

    let mut bb = BindingBuilder::default();
    bb.bind(&self.bump_size);
    bb.bind(&self.current_size);

    bb.setup_compute_pass(pass, device, &pipeline);
    pass.dispatch_workgroups(1, 1, 1);
  }

  pub fn drain_self_into_the_other(
    &self,
    the_other: &Self,
    pass: &mut GPUComputePass,
    device: &GPUDevice,
  ) {
    // let pipeline = compute_shader_builder()
    //   .config_work_group_size(256)
    //   .entry(|cx| {
    //     let input = cx.bind_by(&self.storage);
    //     let output = cx.bind_by(&the_other.storage);

    //     let global_id = cx.global_invocation_id().x();
    //     let local_id = cx.local_invocation_id().x();

    //     let value = input.index(global_id).load().make_local_var();

    //     shared.index(local_id).store(value.load());

    //     output.index(global_id).store(value.load())
    //   })
    //   .create_compute_pipeline(&gpu)
    //   .unwrap();
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
