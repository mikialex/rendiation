use crate::*;

// pub trait DeviceHeapReinterpretation {
//   fn read_from_heap(heap: StorageNodePtr<[u32]>) -> Node<Self>;
//   fn write_into_heap(value: Node<Self>, heap: StorageNodePtr<[u32]>);
// }

/// each thread could generate none deterministic number of result.
///
/// both act as producer and consumer
pub trait DeviceDynamicParallelLogic {
  type Task: Std430 + ShaderSizedValueNodeType;
  type Output: Std430 + ShaderSizedValueNodeType;
  fn task_work(
    task: Node<Self::Task>,
    task_output: &dyn Fn(Node<Self::Task>),
    result_output: &dyn Fn(Node<Self::Output>),
  );
}
pub trait DeviceComputeSize {
  fn device_work_size(&self) -> Node<u32>;
}

pub struct DeviceBumpAllocation<T> {
  capacity: u32,
  ty: PhantomData<T>,
}

pub struct DeviceBumpAllocationInstance<T: Std430> {
  storage: StorageBufferDataView<[T]>,
  bump_size: StorageBufferDataView<DeviceAtomic<u32>>,
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

impl<T: Std430> DeviceComputeSize for DeviceBumpAllocationInvocationInstance<T> {
  fn device_work_size(&self) -> Node<u32> {
    self.bump_size.atomic_load()
  }
}

pub struct DeviceGlobalSpinLock<T> {
  spin_lock: StorageBufferDataView<DeviceAtomic<u32>>,
  inner: T,
}

pub struct DeviceGlobalSpinLockInstance<T> {
  spin_lock: StorageNode<DeviceAtomic<u32>>,
  inner: T,
}

impl<T> DeviceGlobalSpinLockInstance<T> {
  // todo, we should make this type checked
  pub fn access(&self) -> &T {
    &self.inner
  }
  pub fn mutate<R>(&self, mutator: impl Fn(&T) -> (Node<R>, Node<bool>)) -> (Node<R>, Node<bool>)
  where
    R: ShaderSizedValueNodeType,
  {
    let result = zeroed_val().make_local_var();
    let success = val(false).make_local_var();

    let leave_loop = val(false).make_local_var();

    loop_by(|cx| {
      if_by(leave_loop.load(), || cx.do_break());

      let take_lock_success = self.spin_lock.atomic_exchange(val(1)).equals(val(0));
      if_by(take_lock_success, || {
        let (r, r_success) = mutator(&self.inner);
        result.store(r);
        success.store(r_success);

        leave_loop.store(val(true));
        self.spin_lock.atomic_exchange(val(0));
      });
    });

    (result.load(), success.load())
  }
}

pub struct DeviceGlobalVec<T: Std430> {
  storage: StorageBufferDataView<[T]>,
  size: StorageBufferDataView<u32>,
}

pub struct DeviceGlobalVecInstance<T: Std430> {
  storage: StorageNode<[T]>,
  size: StorageNode<u32>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceGlobalVecInstance<T> {
  pub fn len(&self) -> Node<u32> {
    self.size.load()
  }
  pub fn push(&self, item: Node<T>) -> (Node<u32>, Node<bool>) {
    let write_idx = self.size.load() + val(1);
    self.size.store(write_idx);
    let out_of_bound = write_idx.greater_equal_than(self.storage.array_length());
    if_by(out_of_bound.not(), || {
      self.storage.index(write_idx).store(item)
    });
    (write_idx, out_of_bound)
  }
  pub fn pop(&self) -> (Node<T>, Node<bool>) {
    let size = self.size.load();
    let is_not_empty = size.greater_than(0);
    let v = is_not_empty.select_branched(
      || {
        let read_idx = size - val(1);
        self.size.store(read_idx);
        self.storage.index(read_idx).load()
      },
      || zeroed_val(),
    );
    (v, is_not_empty)
  }
}

pub struct DeviceWorkStealingSystem<T: DeviceDynamicParallelLogic> {
  device_global_job_queue: DeviceGlobalSpinLock<DeviceGlobalVec<T::Task>>,

  device_global_result_queue: DeviceBumpAllocation<T::Output>,
  //
}

impl<T: DeviceDynamicParallelLogic> DeviceWorkStealingSystem<T> {
  pub fn execute_workload(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    work: StorageBufferReadOnlyDataView<[T::Task]>,
  ) -> StorageBufferReadOnlyDataView<[T::Output]> {
    todo!()
  }
}

pub struct DeviceWorkStealingSystemInstance<T: DeviceDynamicParallelLogic> {
  device_global_job_queue: DeviceGlobalSpinLockInstance<DeviceGlobalVecInstance<T::Task>>,
  device_global_result_queue: DeviceBumpAllocationInvocationInstance<T::Output>,
  active_thread_count: StorageNode<DeviceAtomic<u32>>,
}

impl<T: DeviceDynamicParallelLogic> DeviceWorkStealingSystemInstance<T> {
  fn thread_logic(&self, builder: ShaderComputePipelineBuilder) {
    builder.entry(|cx| {
      let global_queue_has_work = self.device_global_job_queue.access().len().not_equals(0);

      loop_by(|cx| {
        if_by(global_queue_has_work, || {
          self.active_thread_count.atomic_add(val(1));
          let (work, has_work) = self.device_global_job_queue.mutate(|queue| queue.pop());
          if_by(has_work, || {
            T::task_work(
              work,
              &|new_work| {
                // todo error handling
                let (_, _success) = self
                  .device_global_job_queue
                  .mutate(|queue| queue.push(new_work));
              },
              &|new_result| {
                // todo error handling
                self.device_global_result_queue.allocate(new_result);
              },
            );
          });
          self.active_thread_count.atomic_sub(val(1));
        });

        let no_thread_is_doing_work = self.active_thread_count.atomic_load().equals(val(0));
        let everything_done = global_queue_has_work.not().and(no_thread_is_doing_work);
        if_by(everything_done, || cx.do_break());
      })
    });
  }
}
