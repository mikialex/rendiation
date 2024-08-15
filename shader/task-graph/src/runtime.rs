use crate::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,
  pub state_builder: DynamicTypeBuilder,
}

pub struct DeviceTaskSystemPollCtx<'a> {
  compute_cx: &'a mut ShaderComputePipelineBuilder,
  all_task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  self_task_idx: Node<u32>,
  self_task: TaskPoolInvocationInstance,
  tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
  // the rust hashmap is not ordered
  tasks_depend_on_self_bind_order: Vec<usize>,
}

impl<'a> DeviceTaskSystemPollCtx<'a> {
  // todo, handle self task spawner
  fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> &mut TaskGroupDeviceInvocationInstance {
    self
      .tasks_depend_on_self
      .entry(task_type)
      .or_insert_with(|| {
        let source = &self.all_task_group_sources[task_type];
        self.tasks_depend_on_self_bind_order.push(task_type);
        source.build_shader_for_spawner(self.compute_cx)
      })
  }

  pub fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> StorageNode<T> {
    let current = self.self_task_idx;
    self.self_task.rw_payload(current)
  }

  pub fn spawn_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    argument: Node<T>,
  ) -> TaskFutureInvocationRightValue {
    let task_group = self.get_or_create_task_group_instance(task_type);
    TaskFutureInvocationRightValue {
      task_handle: task_group.spawn_new_task(argument),
    }
  }

  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    let finished = task_group.poll_task_is_finished(task_id);
    if_by(finished, || {
      argument_read_back(task_group.task_pool.rw_payload(task_id).load());
      task_group.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }
}

#[derive(Debug)]
pub struct TaskGraphExecutionStates {
  pub remain_task_counts: Vec<u32>,
}

pub struct DeviceTaskGraphExecutor {
  task_groups: Vec<TaskGroupExecutor>,
  max_recursion_depth: usize,
  current_prepared_execution_size: usize,
}

impl DeviceTaskGraphExecutor {
  pub fn new(current_prepared_execution_size: usize) -> Self {
    Self {
      task_groups: Default::default(),
      max_recursion_depth: 6,
      current_prepared_execution_size,
    }
  }

  pub fn define_task<P, F>(
    &mut self,
    future: F,
    device: &GPUDevice,
    pass: &mut GPUComputePass,
  ) -> u32
  where
    F: DeviceFuture<Output = ()> + 'static,
    P: ShaderSizedValueNodeType,
  {
    self.define_task_inner(
      Box::new(OpaqueTaskWrapper(future)) as OpaqueTask,
      P::sized_ty(),
      device,
      pass,
    )
  }

  pub fn define_task_inner(
    &mut self,
    task: OpaqueTask,
    payload_ty: ShaderSizedValueType,
    device: &GPUDevice,
    pass: &mut GPUComputePass,
  ) -> u32 {
    let task_type = self.task_groups.len();

    let task_group_sources: Vec<_> = self.task_groups.iter().map(|x| &x.resource).collect();

    let mut cx = compute_shader_builder();

    let mut build_ctx = DeviceTaskSystemBuildCtx {
      compute_cx: &mut cx,
      state_builder: DynamicTypeBuilder::new_named(&format!("Task_states_{}", task_type)),
    };

    let state = task.build_poll(&mut build_ctx);

    let state_desc = build_ctx.state_builder.meta_info();

    let mut task_type_desc = ShaderStructMetaInfo::new("TaskType");
    task_type_desc.push_field_dyn(
      "is_finished",
      ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Uint32),
    );
    task_type_desc.push_field_dyn("payload", payload_ty);
    task_type_desc.push_field_dyn("state", ShaderSizedValueType::Struct(state_desc.ty.clone()));
    let mut state_builder = build_ctx.state_builder;

    let outer_builder = take_build_api(); // workaround, should be improved?
    let resource = TaskGroupExecutorResource::create_with_size(
      self.current_prepared_execution_size,
      state_desc.clone(),
      task_type_desc.clone(),
      device,
      pass,
    );
    set_build_api(outer_builder);

    let indices = cx.bind_by(&resource.alive_task_idx.storage);
    let task_index = indices.index(cx.global_invocation_id().x()).load();

    let pool = resource.task_pool.build_shader(&mut cx);
    let item = pool.access_item_ptr(task_index);
    state_builder.resolve(item.cast_untyped_node());

    let mut ctx = DeviceTaskSystemPollCtx {
      all_task_group_sources: task_group_sources,
      tasks_depend_on_self: Default::default(),
      tasks_depend_on_self_bind_order: Default::default(),
      self_task_idx: task_index,
      self_task: pool.clone(),
      compute_cx: &mut cx,
    };

    let poll_result = state.device_poll(&mut ctx);
    if_by(poll_result.is_ready, || {
      pool.rw_is_finished(task_index).store(0);
    });

    let tasks_depend_on_self = ctx.tasks_depend_on_self_bind_order;
    let polling_pipeline = cx.create_compute_pipeline(device).unwrap();

    let task_executor = TaskGroupExecutor {
      polling_pipeline,
      resource,
      state_desc,
      task_type_desc,
      tasks_depend_on_self,
      required_poll_count: task.required_poll_count(),
      task,
    };
    self.task_groups.push(task_executor);

    task_type as u32
  }

  /// set exact execution dispatch size for this executor, this will resize all resources
  pub fn set_execution_size(
    &mut self,
    gpu: &GPU,
    ctx: &mut DeviceParallelComputeCtx,
    dispatch_size: usize,
  ) {
    let dispatch_size = dispatch_size.min(1);
    if self.current_prepared_execution_size == dispatch_size {
      return;
    }
    self.current_prepared_execution_size = dispatch_size;
    for s in &mut self.task_groups {
      s.task.reset(ctx, dispatch_size as u32);
      ctx.record_pass(|pass, _| {
        s.resource.resize(
          gpu,
          dispatch_size,
          self.max_recursion_depth,
          s.state_desc.clone(),
          s.task_type_desc.clone(),
          pass,
        )
      })
    }
  }

  pub fn make_sure_execution_size_is_enough(
    &mut self,
    gpu: &GPU,
    ctx: &mut DeviceParallelComputeCtx,
    dispatch_size: usize,
  ) {
    let is_contained = self.current_prepared_execution_size <= dispatch_size;

    if !is_contained {
      self.set_execution_size(gpu, ctx, dispatch_size)
    }
  }

  /// Allocate task directly in the task pool by dispatching compute shader.
  ///
  /// T must match given task_id's payload type
  ///
  /// From perspective of performance, this method can be implemented as a special task
  /// polling, but for consistency and simplicity, we implemented as a standalone task allocation procedure.
  pub fn dispatch_allocate_init_task<T: ShaderSizedValueNodeType>(
    &mut self,
    device: &GPUDevice,
    pass: &mut GPUComputePass,
    dispatch_size: u32,
    task_id: u32,
    task_spawner: impl FnOnce(Node<u32>) -> Node<T> + 'static,
  ) {
    let task_group = &self.task_groups[task_id as usize];

    let size_range = create_gpu_readonly_storage(&dispatch_size, device);

    let hasher = PipelineHasher::default().with_hash(task_spawner.type_id());
    let workgroup_size = 256;
    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
      builder.config_work_group_size(workgroup_size);

      let size_range = builder.bind_by(&size_range);
      let instance = task_group.resource.build_shader_for_spawner(&mut builder);
      let id = builder.global_invocation_id().x();
      let payload = task_spawner(id);

      if_by(id.less_than(size_range.load()), || {
        instance.spawn_new_task(payload);
      });

      builder
    });

    let mut bb = BindingBuilder::new_as_compute().with_bind(&size_range);
    task_group.resource.bind_for_spawner(&mut bb);
    bb.setup_compute_pass(pass, device, &pipeline);

    let size = compute_dispatch_size(dispatch_size, workgroup_size);
    pass.dispatch_workgroups(size, 1, 1);

    task_group
      .resource
      .alive_task_idx
      .commit_size(pass, device, true);
    task_group
      .resource
      .empty_index_pool
      .commit_size(pass, device, false);
  }

  pub async fn read_back_execution_states<'a>(
    &mut self,
    cx: &mut DeviceParallelComputeCtx<'a>,
  ) -> TaskGraphExecutionStates {
    cx.flush_pass();

    let result_size = self.task_groups.len() * 4;
    let result_size = NonZeroU64::new(result_size as u64).unwrap();
    let result_buffer =
      create_gpu_read_write_storage::<u32>(StorageBufferInit::Zeroed(result_size), &cx.gpu.device);

    for (i, task) in self.task_groups.iter().enumerate() {
      cx.encoder.copy_buffer_to_buffer(
        task.resource.alive_task_idx.current_size.buffer.gpu(),
        0,
        result_buffer.buffer.gpu(),
        i as u64 * 4,
        4,
      );
    }

    let result = cx.encoder.read_buffer(&cx.gpu.device, &result_buffer);

    cx.submit_recorded_work_and_continue();

    let buffer = result.await.unwrap();

    let results = <[u32]>::from_bytes_into_boxed(&buffer.read_raw()).into_vec();

    TaskGraphExecutionStates {
      remain_task_counts: results,
    }
  }

  // todo, impl more conservative upper execute bound
  pub fn compute_conservative_dispatch_round_count(&self) -> usize {
    let max_required_poll_count = self
      .task_groups
      .iter()
      .map(|v| v.required_poll_count)
      .max()
      .unwrap_or(1);
    self.max_recursion_depth * max_required_poll_count + 1
  }

  pub fn execute(&mut self, cx: &mut DeviceParallelComputeCtx, dispatch_round_count: usize) {
    // it's safe because the reference is not overlapped
    let self_task_groups: &[TaskGroupExecutor] = &self.task_groups;
    let self_task_groups: &'static [TaskGroupExecutor] =
      unsafe { std::mem::transmute(self_task_groups) };

    for _ in 0..dispatch_round_count {
      for task in &mut self.task_groups {
        task.execute(cx, self_task_groups);
      }
    }
  }
}

type OpaqueTask = Box<
  dyn DeviceFuture<
    Output = Box<dyn Any>,
    Invocation = Box<dyn DeviceFutureInvocation<Output = Box<dyn Any>>>,
  >,
>;

struct TaskGroupExecutor {
  state_desc: DynamicTypeMetaInfo,
  task_type_desc: ShaderStructMetaInfo,
  task: OpaqueTask,

  polling_pipeline: GPUComputePipeline,
  tasks_depend_on_self: Vec<usize>,
  resource: TaskGroupExecutorResource,
  required_poll_count: usize,
}

impl TaskGroupExecutor {
  pub fn execute(&mut self, cx: &mut DeviceParallelComputeCtx, all_tasks: &[Self]) {
    cx.record_pass(|pass, device| {
      self.resource.alive_task_idx.commit_size(pass, device, true);
    });

    {
      let imp = &mut self.resource;
      // compact active task buffer
      let alive_tasks = imp.alive_task_idx.storage.clone().into_readonly_view();
      let size = imp.alive_task_idx.current_size.clone();
      imp.alive_task_idx.storage = alive_tasks
        .clone()
        .stream_compaction(ActiveTaskCompact {
          alive_size: size,
          active_tasks: alive_tasks.clone(),
          task_pool: imp.task_pool.clone(),
        })
        .materialize_storage_buffer(cx)
        .buffer
        .into_rw_view();

      cx.record_pass(|pass, device| {
        let imp = &mut self.resource;
        // update alive task count
        {
          let hasher = shader_hasher_from_marker_ty!(SizeUpdate);
          let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
            builder.config_work_group_size(1);
            let bump_size = builder.bind_by(&imp.new_removed_task_idx.bump_size);
            let current_size = builder.bind_by(&imp.alive_task_idx.current_size);
            let delta = bump_size.atomic_load();
            current_size.store(current_size.load() - delta);
            builder
          });

          BindingBuilder::new_as_compute()
            .with_bind(&imp.new_removed_task_idx.bump_size)
            .with_bind(&imp.alive_task_idx.current_size)
            .setup_compute_pass(pass, device, &pipeline);

          pass.dispatch_workgroups(1, 1, 1);
        }
        // commit other bumper size
        imp.empty_index_pool.commit_size(pass, device, false);
        imp.new_removed_task_idx.commit_size(pass, device, true);
      });
    }

    cx.record_pass(|pass, device| {
      let imp = &mut self.resource;
      // drain empty to empty pool
      let size =
        imp
          .new_removed_task_idx
          .drain_self_into_the_other(&imp.empty_index_pool, pass, device);

      // dispatch tasks
      let mut bb = BindingBuilder::new_as_compute().with_bind(&imp.alive_task_idx.storage);

      for task in &self.tasks_depend_on_self {
        let task = &all_tasks[*task];
        task.resource.bind_for_spawner(&mut bb);
      }
      imp.task_pool.bind(&mut bb);

      bb.setup_compute_pass(pass, device, &self.polling_pipeline);
      pass.dispatch_workgroups_indirect_owned(&size);
    });
  }
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
    task_ty_desc: ShaderStructMetaInfo,
    device: &GPUDevice,
    pass: &mut GPUComputePass,
  ) -> Self {
    let res = Self {
      alive_task_idx: DeviceBumpAllocationInstance::new(size * 2, device),
      new_removed_task_idx: DeviceBumpAllocationInstance::new(size, device),
      empty_index_pool: DeviceBumpAllocationInstance::new(size * 2, device),
      task_pool: TaskPool::create_with_size(size * 2, state_desc, task_ty_desc, device),
      size,
    };

    let hasher = shader_hasher_from_marker_ty!(PrepareEmptyIndices);

    let workgroup_size = 256;
    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
      builder.config_work_group_size(workgroup_size);

      let empty_pool = builder.bind_by(&res.empty_index_pool.storage);
      let empty_pool_size = builder.bind_by(&res.empty_index_pool.current_size);
      let id = builder.global_invocation_id().x();

      if_by(id.equals(0), || {
        empty_pool_size.store(empty_pool.array_length());
      });

      empty_pool.index(id).store(id);
      builder
    });

    BindingBuilder::new_as_compute()
      .with_bind(&res.empty_index_pool.storage)
      .with_bind(&res.empty_index_pool.current_size)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(compute_dispatch_size(size as u32 * 2, workgroup_size), 1, 1);

    res
  }

  fn resize(
    &mut self,
    gpu: &GPU,
    size: usize,
    max_recursion_depth: usize,
    state_desc: DynamicTypeMetaInfo,
    task_ty_desc: ShaderStructMetaInfo,
    pass: &mut GPUComputePass,
  ) {
    if self.size != size * max_recursion_depth {
      *self = Self::create_with_size(size, state_desc, task_ty_desc, &gpu.device, pass);
    }
  }

  fn build_shader_for_spawner(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> TaskGroupDeviceInvocationInstance {
    TaskGroupDeviceInvocationInstance {
      new_removed_task_idx: self.new_removed_task_idx.build_allocator_shader(cx),
      empty_index_pool: self.empty_index_pool.build_deallocator_shader(cx),
      task_pool: self.task_pool.build_shader(cx),
      alive_task_idx: self.alive_task_idx.build_allocator_shader(cx),
    }
  }

  fn bind_for_spawner(&self, cx: &mut BindingBuilder) {
    self.new_removed_task_idx.bind_allocator(cx);
    self.empty_index_pool.bind_allocator(cx);
    self.task_pool.bind(cx);
    self.alive_task_idx.bind_allocator(cx);
  }
}

#[derive(Clone)]
struct TaskPool {
  /// struct Task {
  ///   is_finished_ bool,
  ///   payload: P,
  ///   state: S,
  /// }
  tasks: GPUBufferResourceView,
  state_desc: DynamicTypeMetaInfo,
  task_ty_desc: ShaderStructMetaInfo,
}

impl TaskPool {
  pub fn create_with_size(
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    task_ty_desc: ShaderStructMetaInfo,
    device: &GPUDevice,
  ) -> Self {
    let usage = BufferUsages::STORAGE;

    let stride = task_ty_desc.size_of_self(StructLayoutTarget::Std430);

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
      task_ty_desc,
    }
  }

  pub fn build_shader(&self, cx: &mut ShaderComputePipelineBuilder) -> TaskPoolInvocationInstance {
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ShaderValueSingleType::Unsized(
        ShaderUnSizedValueType::UnsizedArray(Box::new(ShaderSizedValueType::Struct(
          self.task_ty_desc.clone(),
        ))),
      )),
      writeable_if_storage: true,
    };
    let node = cx.bindgroups().binding_dyn(desc).compute_node;
    TaskPoolInvocationInstance {
      pool: unsafe { node.into_node() },
      state_desc: self.state_desc.clone(),
      payload_ty: self.task_ty_desc.fields[1].ty.clone(),
    }
  }

  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind_dyn(
      self.tasks.get_binding_build_source(),
      &ShaderBindingDescriptor {
        should_as_storage_buffer_if_is_buffer_like: true,
        ty: ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(
          self.state_desc.ty.clone(),
        ))),
        writeable_if_storage: true,
      },
    );
  }
}

#[derive(Clone)]
struct ActiveTaskCompact {
  alive_size: StorageBufferDataView<u32>,
  active_tasks: StorageBufferReadOnlyDataView<[u32]>,
  task_pool: TaskPool,
}

impl DeviceParallelCompute<Node<bool>> for ActiveTaskCompact {
  fn execute_and_expose(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<bool>>> {
    Box::new(self.clone())
  }

  fn result_size(&self) -> u32 {
    self.active_tasks.item_count()
  }
}
impl DeviceParallelComputeIO<bool> for ActiveTaskCompact {}

impl ShaderHashProvider for ActiveTaskCompact {
  shader_hash_type_id! {}
}

impl DeviceInvocationComponent<Node<bool>> for ActiveTaskCompact {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<bool>>> {
    let active_tasks = builder.bind_by(&self.active_tasks);
    let size = builder.bind_by(&self.alive_size);
    let task_pool = self.task_pool.build_shader(builder);
    let inner = (active_tasks, size, task_pool);

    RealAdhocInvocationResult {
      inner,
      compute: Box::new(|inner, id| {
        let (r, is_valid) = inner.0.invocation_logic(id);

        //  check task_pool access is valid?
        let r = inner.2.poll_task_is_finished(r);

        (r, is_valid)
      }),
      size: Box::new(|inner| (inner.1.load(), val(0), val(0)).into()),
    }
    .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.active_tasks);
    builder.bind(&self.alive_size);
    self.task_pool.bind(builder);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn work_size(&self) -> Option<u32> {
    None
  }
}

pub struct TaskGroupDeviceInvocationInstance {
  new_removed_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  empty_index_pool: DeviceBumpDeAllocationInvocationInstance<u32>,
  alive_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  task_pool: TaskPoolInvocationInstance,
}

impl TaskGroupDeviceInvocationInstance {
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(&self, payload: Node<T>) -> Node<u32> {
    let (idx, success) = self.empty_index_pool.bump_deallocate();
    if_by(success, || {
      self.task_pool.spawn_new_task(idx, payload);
      self.alive_task_idx.bump_allocate(idx);
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
  payload_ty: ShaderSizedValueType,
}

impl TaskPoolInvocationInstance {
  fn access_item_ptr(&self, idx: Node<u32>) -> StorageNode<AnyType> {
    self.pool.index(idx)
  }

  pub fn poll_task_is_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self.rw_is_finished(task_id).load().not_equals(0)
  }
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(&self, at: Node<u32>, payload: Node<T>) {
    self.rw_is_finished(at).store(1);

    self.rw_payload(at).store(payload);

    assert_eq!(self.payload_ty, T::sized_ty());

    // write states with given init value
    let state_ptr = self.rw_states(at);
    for (i, v) in self.state_desc.fields_init.iter().enumerate() {
      unsafe {
        let state_field: StorageNode<AnyType> = index_access_field(state_ptr.handle(), i);
        state_field.store(
          v.to_shader_node_by_value(&self.state_desc.ty.fields[i].ty)
            .into_node(),
        );
      };
    }
  }

  pub fn rw_is_finished(&self, task: Node<u32>) -> StorageNode<u32> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 0) }
  }
  pub fn rw_payload<T: ShaderNodeType>(&self, task: Node<u32>) -> StorageNode<T> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 1) }
  }
  pub fn rw_states(&self, task: Node<u32>) -> StorageNode<AnyType> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 2) }
  }
}
