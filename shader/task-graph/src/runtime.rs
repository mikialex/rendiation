use crate::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  all_task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  self_task_idx: Node<u32>,
  self_task: TaskPoolInvocationInstance,
  tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
}

impl<'a> DeviceTaskSystemBuildCtx<'a> {
  fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
    ccx: &mut ComputeCx,
  ) -> &mut TaskGroupDeviceInvocationInstance {
    self
      .tasks_depend_on_self
      .entry(task_type)
      .or_insert_with(|| {
        let source = &self.all_task_group_sources[task_type];
        source.build_shader(ccx)
      })
  }

  pub fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> StorageNode<T> {
    let current = self.self_task_idx;
    self.self_task.read_payload(current)
  }

  pub fn spawn_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    argument: Node<T>,
    cx: &mut ComputeCx,
  ) -> Node<u32> {
    let task_group = self.get_or_create_task_group_instance(task_type, cx);
    task_group.spawn_new_task(argument)
  }

  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
    cx: &mut ComputeCx,
  ) -> Node<bool> {
    let task_group = self.get_or_create_task_group_instance(task_type, cx);
    let finished = task_group.poll_task_is_finished(task_id);
    if_by(finished, || {
      argument_read_back(task_group.task_pool.read_payload(task_id).load());
      task_group.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }
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

  pub fn define_task<F>(
    &mut self,
    future: F,
    f_ctx: impl FnOnce() -> F::Ctx,
    device: &GPUDevice,
    pass: &mut GPUComputePass,
  ) -> u32
  where
    F: DeviceFuture<Output = ()>,
  {
    let task_type = self.task_groups.len();

    let task_group_sources: Vec<_> = self.task_groups.iter().map(|x| &x.resource).collect();

    let mut state_builder = DynamicTypeBuilder::new_named(&format!("Task_states_{}", task_type));
    let state = future.create_or_reconstruct_state(&mut state_builder);

    let state_desc = state_builder.meta_info();

    let resource = TaskGroupExecutorResource::create_with_size(
      self.current_prepared_execution_size,
      state_desc.clone(),
      device,
      pass,
    );

    let mut compute_cx = compute_shader_builder();
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

    let mut ctx = DeviceTaskSystemBuildCtx {
      all_task_group_sources: task_group_sources,
      tasks_depend_on_self: Default::default(),
      self_task_idx: task_index,
      self_task: task_pool.clone(),
    };
    let mut f_ctx = f_ctx();

    compute_cx.entry_by(|ccx| {
      let poll_result = future.poll(&state, ccx, &mut ctx, &mut f_ctx);
      if_by(poll_result.is_ready, || {
        task_pool.read_is_finished(task_index).store(true);
      });
    });

    let task_poll_pipeline = compute_cx.create_compute_pipeline(device).unwrap();

    let task_executor = TaskGroupExecutor {
      polling_pipeline: task_poll_pipeline,
      resource,
      state_desc,
      required_poll_count: future.required_poll_count(),
    };
    self.task_groups.push(task_executor);

    task_type as u32
  }

  /// set exact execution dispatch size for this executor, this will resize all resources
  pub fn set_execution_size(&mut self, gpu: &GPU, pass: &mut GPUComputePass, dispatch_size: usize) {
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
        s.state_desc.clone(),
        pass,
      )
    }
  }

  pub fn make_sure_execution_size_is_enough(
    &mut self,
    gpu: &GPU,
    pass: &mut GPUComputePass,
    dispatch_size: usize,
  ) {
    let is_contained = self.current_prepared_execution_size <= dispatch_size;

    if !is_contained {
      self.set_execution_size(gpu, pass, dispatch_size)
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
    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |builder| {
      builder.config_work_group_size(workgroup_size).entry(|cx| {
        let size_range = cx.bind_by(&size_range);
        let instance = task_group.resource.build_shader(cx);
        let id = cx.global_invocation_id().x();
        let payload = task_spawner(id);

        if_by(id.less_than(size_range.load()), || {
          instance.spawn_new_task(payload);
        });
      })
    });

    let mut bb = BindingBuilder::new_as_compute().with_bind(&size_range);
    task_group.resource.bind(&mut bb);
    bb.setup_compute_pass(pass, device, &pipeline);

    let size = compute_dispatch_size(dispatch_size, workgroup_size);
    pass.dispatch_workgroups(size, 1, 1);
  }

  pub fn execute(&mut self, cx: &mut DeviceParallelComputeCtx) {
    let max_required_poll_count = self
      .task_groups
      .iter()
      .map(|v| v.required_poll_count)
      .max()
      .unwrap_or(1);
    let required_round = self.max_recursion_depth * max_required_poll_count + 1;

    // todo, impl more conservative upper execute bound
    for _ in 0..required_round {
      for stage in &mut self.task_groups {
        stage.execute(cx);
      }
    }
    // todo check state states to make sure no task remains
  }
}

struct TaskGroupExecutor {
  state_desc: DynamicTypeMetaInfo,
  polling_pipeline: GPUComputePipeline,
  resource: TaskGroupExecutorResource,
  required_poll_count: usize,
}

impl TaskGroupExecutor {
  pub fn execute(&mut self, cx: &mut DeviceParallelComputeCtx) {
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
        .into_rw_view();

      cx.record_pass(|pass, device| {
        let imp = &mut self.resource;
        // update alive task count
        {
          let hasher = shader_hasher_from_marker_ty!(SizeUpdate);
          let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |builder| {
            builder.config_work_group_size(1).entry(|cx| {
              let bump_size = cx.bind_by(&imp.new_removed_task_idx.bump_size);
              let current_size = cx.bind_by(&imp.alive_task_idx.current_size);
              let delta = bump_size.atomic_load();
              current_size.store(current_size.load() - delta);
            })
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
      imp
        .new_removed_task_idx
        .drain_self_into_the_other(&imp.empty_index_pool, pass, device);

      // dispatch tasks
      let mut bb = BindingBuilder::new_as_compute();
      imp.bind(&mut bb);
      bb.setup_compute_pass(pass, device, &self.polling_pipeline);
      let size = imp.alive_task_idx.prepare_dispatch_size(pass, device, 64);
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
    device: &GPUDevice,
    pass: &mut GPUComputePass,
  ) -> Self {
    let res = Self {
      alive_task_idx: DeviceBumpAllocationInstance::new(size * 2, device),
      new_removed_task_idx: DeviceBumpAllocationInstance::new(size, device),
      empty_index_pool: DeviceBumpAllocationInstance::new(size * 2, device),
      task_pool: TaskPool::create_with_size(size * 2, state_desc, device),
      size,
    };

    let hasher = shader_hasher_from_marker_ty!(PrepareEmptyIndices);

    let workgroup_size = 256;
    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |builder| {
      builder.config_work_group_size(workgroup_size).entry(|cx| {
        let empty_pool = cx.bind_by(&res.empty_index_pool.storage);
        let empty_pool_size = cx.bind_by(&res.empty_index_pool.current_size);
        let id = cx.global_invocation_id().x();

        if_by(id.equals(0), || {
          empty_pool_size.store(empty_pool.array_length());
        });

        empty_pool.index(id).store(id);
      })
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
    pass: &mut GPUComputePass,
  ) {
    if self.size != size * max_recursion_depth {
      *self = Self::create_with_size(size, state_desc, &gpu.device, pass);
    }
  }

  fn build_shader(&self, cx: &mut ComputeCx) -> TaskGroupDeviceInvocationInstance {
    TaskGroupDeviceInvocationInstance {
      new_removed_task_idx: self.new_removed_task_idx.build_allocator_shader(cx),
      empty_index_pool: self.empty_index_pool.build_deallocator_shader(cx),
      task_pool: self.task_pool.build_shader(cx),
      alive_task_idx: self.alive_task_idx.build_allocator_shader(cx),
    }
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    self.new_removed_task_idx.bind_allocator(cx);
    self.empty_index_pool.bind_allocator(cx);
    self.task_pool.bind(cx);
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
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ShaderValueSingleType::Unsized(
        ShaderUnSizedValueType::UnsizedArray(Box::new(ShaderSizedValueType::Struct(
          self.state_desc.ty.clone(),
        ))),
      )),
      writeable_if_storage: true,
    };
    let node = cx.bindgroups().binding_dyn(desc).compute_node;
    TaskPoolInvocationInstance {
      pool: unsafe { node.into_node() },
      state_desc: self.state_desc.clone(),
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
    let r: AdhocInvocationResult<Node<bool>> = builder.entry_by(|cx| {
      let active_tasks = cx.bind_by(&self.active_tasks);
      let size = cx.bind_by(&self.alive_size);
      let size = (size.load(), val(0), val(0)).into();
      let task_pool = self.task_pool.build_shader(cx);

      let (r, is_valid) = active_tasks.invocation_logic(cx.global_invocation_id());

      //  check task_pool access is valid?
      let r = task_pool.read_is_finished(r).load();
      AdhocInvocationResult(size, r, is_valid)
    });

    Box::new(r)
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
        let state_field: StorageNode<AnyType> = index_access_field(state_ptr.handle(), i);
        state_field.store(v.into_raw_node().into_node());
      };
    }
  }

  pub fn read_is_finished(&self, task: Node<u32>) -> StorageNode<bool> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 0) }
  }
  pub fn read_payload<T: ShaderNodeType>(&self, task: Node<u32>) -> StorageNode<T> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 1) }
  }
  pub fn read_states(&self, task: Node<u32>) -> StorageNode<AnyType> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 2) }
  }
}
