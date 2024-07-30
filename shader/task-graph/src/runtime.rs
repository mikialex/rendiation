use crate::*;

pub struct DeviceTaskGraphExecutor {
  task_groups: Vec<TaskGroupExecutor>,
  max_recursion_depth: usize,
  current_prepared_execution_size: usize,
}

pub struct DeviceTaskSystemBuildCtx<'a> {
  inner: RwLock<DeviceTaskSystemBuildCtxImpl<'a>>,
}

pub struct DeviceTaskSystemBuildCtxImpl<'a> {
  compute_cx: &'a mut ShaderComputePipelineBuilder,
  task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  current_task_idx: Node<u32>,
  self_task: TaskPoolInvocationInstance,
  tasks: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
}

impl<'a> DeviceTaskSystemBuildCtxImpl<'a> {
  fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> &mut TaskGroupDeviceInvocationInstance {
    self.tasks.entry(task_type).or_insert_with(|| {
      let source = &self.task_group_sources[task_type];
      source.build_shader(self.compute_cx)
    })
  }

  pub fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> StorageNode<T> {
    let current = self.current_task_idx;
    self.self_task.read_payload(current)
  }

  fn spawn_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    argument: Node<T>,
  ) -> Node<u32> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    task_group.spawn_new_task(argument)
  }

  fn poll_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    let finished = task_group.poll_task_is_finished(task_id);
    if_by(finished, || {
      argument_read_back(task_group.task_pool.read_payload(task_id).load());
      task_group.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }
}

impl<'a> DeviceTaskSystemContextProvider for DeviceTaskSystemBuildCtx<'a> {
  fn spawn_task<T: ShaderSizedValueNodeType>(
    &self,
    task_type: usize,
    argument: Node<T>,
  ) -> Node<u32> {
    self.inner.write().spawn_task(task_type, argument)
  }

  fn poll_task<T: ShaderSizedValueNodeType>(
    &self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    self
      .inner
      .write()
      .poll_task(task_type, task_id, argument_read_back)
  }
}

impl DeviceTaskGraphExecutor {
  pub fn empty() -> Self {
    Self {
      task_groups: Default::default(),
      max_recursion_depth: 6,
      current_prepared_execution_size: 128,
    }
  }

  pub fn define_task<F>(
    &mut self,
    future: F,
    cx_provider: impl FnOnce(&mut DeviceTaskSystemBuildCtx) -> F::Ctx,
    device: &GPUDevice,
    init_size: usize,
  ) -> u32
  where
    F: DeviceFuture<Output = ()>,
  {
    let mut compute_cx = compute_shader_builder();
    let task_type = self.task_groups.len();

    let task_group_sources: Vec<_> = self.task_groups.iter().map(|x| &x.resource).collect();

    let mut state_builder = DynamicTypeBuilder::new_named(&format!("Task_states_{}", task_type));
    let state = future.create_or_reconstruct_state(&mut state_builder);

    let state_desc = state_builder.meta_info();

    let resource =
      TaskGroupExecutorResource::create_with_size(init_size, state_desc.clone(), device);

    let task_index = compute_cx.entry_by(|cx| {
      let indices = cx.bind_by(&resource.alive_task_idx.storage);
      indices.index(cx.global_invocation_id().x()).load()
    });

    let task_pool = compute_cx.entry_by(|cx| {
      let pool = resource.task_pool.build_shader(cx);
      let item = pool.access_item_ptr(task_index);
      state_builder.resolve(item.cast_untyped_node());
      pool
    });

    let ctx = DeviceTaskSystemBuildCtxImpl {
      compute_cx: &mut compute_cx,
      task_group_sources,
      tasks: Default::default(),
      current_task_idx: task_index,
      self_task: task_pool.clone(),
    };
    let mut ctx = DeviceTaskSystemBuildCtx {
      inner: RwLock::new(ctx),
    };
    let ctx = cx_provider(&mut ctx);

    let task_poll_pipeline = compute_cx
      .entry(|_| {
        let poll_result = future.poll(&state, &ctx);
        if_by(poll_result.is_ready, || {
          task_pool.read_is_finished(task_index).store(true);
        });
      })
      .create_compute_pipeline(device)
      .unwrap();

    let task_executor = TaskGroupExecutor {
      task_poll_pipeline,
      resource,
      task_state_desc: state_desc,
    };
    self.task_groups.push(task_executor);

    task_type as u32
  }
}

impl DeviceTaskGraphExecutor {
  pub fn set_execution_size(&mut self, gpu: &GPU, dispatch_size: usize) {
    let dispatch_size = dispatch_size.min(1);
    if self.current_prepared_execution_size == dispatch_size {
      return;
    }
    self.current_prepared_execution_size = dispatch_size;
    for s in &mut self.task_groups {
      s.resource.resize(
        gpu,
        dispatch_size,
        self.max_recursion_depth,
        s.task_state_desc.clone(),
      )
    }
  }

  fn make_sure_execution_size_is_enough(&mut self, gpu: &GPU, dispatch_size: usize) {
    let is_contained = self.current_prepared_execution_size <= dispatch_size;

    if !is_contained {
      self.set_execution_size(gpu, dispatch_size)
    }
  }
}

impl DeviceTaskGraphExecutor {
  pub fn execute(&mut self, gpu: &GPU, dispatch_size: usize) {
    self.make_sure_execution_size_is_enough(gpu, dispatch_size);

    let mut encoder = gpu.create_encoder();

    encoder.compute_pass_scoped(|mut pass| {
      for _ in 0..self.max_recursion_depth {
        for stage in &self.task_groups {
          stage.execute(&mut pass);
        }
      }
    });
    // todo check state states to make sure no task remains
  }
}

struct TaskGroupExecutor {
  task_state_desc: DynamicTypeMetaInfo,
  task_poll_pipeline: GPUComputePipeline,
  resource: TaskGroupExecutorResource,
}

struct TaskGroupExecutorResource {
  alive_task_idx: DeviceBumpAllocationInstance<u32>,
  // main_dispatch_size: StorageBufferDataView<DispatchIndirectArgs>,
  new_removed_task_idx: DeviceBumpAllocationInstance<u32>,
  empty_index_pool: DeviceBumpAllocationInstance<u32>,
  task_pool: TaskPool,
  size: usize,
}

impl TaskGroupExecutorResource {
  pub fn create_with_size(
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    device: &GPUDevice,
  ) -> Self {
    Self {
      alive_task_idx: DeviceBumpAllocationInstance::new(size, device),
      // main_dispatch_size: (),
      new_removed_task_idx: DeviceBumpAllocationInstance::new(size, device),
      empty_index_pool: DeviceBumpAllocationInstance::new(size, device),
      task_pool: TaskPool::create_with_size(size, state_desc, device),
      size,
    }
  }
}

struct TaskPool {
  /// struct Task {
  ///   is_finished_ bool,
  ///   payload: P,
  ///   state: S,
  /// }
  tasks: GPUBufferResourceView,
  state_desc: DynamicTypeMetaInfo,
}

impl TaskPool {
  pub fn create_with_size(
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    device: &GPUDevice,
  ) -> Self {
    let usage = BufferUsages::STORAGE;

    let stride = state_desc.ty.size_of_self(StructLayoutTarget::Std430);

    let init = BufferInit::Zeroed(NonZeroU64::new((size * stride) as u64).unwrap());
    let desc = GPUBufferDescriptor {
      size: init.size(),
      usage,
    };

    let gpu = GPUBuffer::create(device, init, usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, desc, device).create_default_view();

    Self {
      tasks: gpu,
      state_desc,
    }
  }

  pub fn build_shader(&self, cx: &mut ComputeCx) -> TaskPoolInvocationInstance {
    let desc = todo!();
    // ShaderBindingDescriptor {
    //   should_as_storage_buffer_if_is_buffer_like: true,
    //   ty: ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(
    //     todo!(),
    //   ))),
    //   writeable_if_storage: true,
    // }
    let node = cx.bindgroups().binding_dyn(desc).compute_node;
    TaskPoolInvocationInstance {
      pool: unsafe { node.into_node() },
      state_desc: self.state_desc.clone(),
    }
  }
}

impl TaskGroupExecutor {
  pub fn execute(&self, pass: &mut GPUComputePass) {
    pass.set_pipeline_owned(&self.task_poll_pipeline);
    // pass.set_bind_group(index, bind_group, offsets)
    // todo, prepare size args
    // pass.dispatch_workgroups_indirect(&self.main_dispatch_size.buffer.gpu(), 0);
  }
}

impl TaskGroupExecutorResource {
  pub fn resize(
    &mut self,
    gpu: &GPU,
    size: usize,
    max_recursion_depth: usize,
    state_desc: DynamicTypeMetaInfo,
  ) {
    if self.size != size * max_recursion_depth {
      *self = Self::create_with_size(size, state_desc, &gpu.device);
    }
  }
  pub fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> TaskGroupDeviceInvocationInstance {
    TaskGroupDeviceInvocationInstance {
      new_removed_task_idx: self.new_removed_task_idx.build_allocator_shader(compute_cx),
      empty_index_pool: self.empty_index_pool.build_deallocator_shader(compute_cx),
      task_pool: compute_cx.entry_by(|cx| self.task_pool.build_shader(cx)),
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DispatchIndirectArgs {
  pub x: u32,
  pub y: u32,
  pub z: u32,
}

pub struct TaskGroupDeviceInvocationInstance {
  new_removed_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  empty_index_pool: DeviceBumpDeAllocationInvocationInstance<u32>,
  task_pool: TaskPoolInvocationInstance,
}

impl TaskGroupDeviceInvocationInstance {
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(&self, payload: Node<T>) -> Node<u32> {
    let (idx, success) = self.empty_index_pool.bump_deallocate();
    if_by(success, || {
      self.task_pool.spawn_new_task(idx, payload);
    })
    .else_by(|| {
      // error report, theoretically unreachable
    });
    idx
  }

  pub fn cleanup_finished_task_state_and_payload(&self, task: Node<u32>) {
    let (_, success) = self.new_removed_task_idx.bump_allocate(task);
    if_by(success.not(), || {
      // error report, theoretically unreachable
    });
  }

  pub fn poll_task_is_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self.task_pool.poll_task_is_finished(task_id)
  }
}

#[derive(Clone)]
struct TaskPoolInvocationInstance {
  pool: StorageNode<[AnyType]>,
  state_desc: DynamicTypeMetaInfo,
}

impl TaskPoolInvocationInstance {
  fn access_item_ptr(&self, idx: Node<u32>) -> StorageNode<AnyType> {
    self.pool.index(idx)
  }

  pub fn poll_task_is_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self.read_is_finished(task_id).load()
  }
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(&self, at: Node<u32>, payload: Node<T>) {
    self.read_is_finished(at).store(false);

    self.read_payload(at).store(payload);

    // write states with given init value
    let state_ptr = self.read_states(at);
    for (i, v) in self.state_desc.fields_init.iter().enumerate() {
      unsafe {
        let state_field: StorageNode<AnyType> = expand_single(state_ptr.handle(), i);
        state_field.store(v.into_raw_node().into_node());
      };
    }
  }

  pub fn read_payload<T: ShaderNodeType>(&self, task: Node<u32>) -> StorageNode<T> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { expand_single(item_ptr.handle(), 1) }
  }
  pub fn read_states(&self, task: Node<u32>) -> StorageNode<AnyType> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { expand_single(item_ptr.handle(), 2) }
  }

  pub fn read_is_finished(&self, task: Node<u32>) -> StorageNode<bool> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { expand_single(item_ptr.handle(), 0) }
  }
}
