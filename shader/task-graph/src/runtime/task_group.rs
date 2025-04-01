use super::*;

pub type OpaqueTask = Box<
  dyn ShaderFuture<
    Output = Box<dyn Any>,
    Invocation = Box<dyn ShaderFutureInvocation<Output = Box<dyn Any>>>,
  >,
>;

pub struct TaskGroupExecutor {
  pub state_desc: DynamicTypeMetaInfo,
  pub max_in_flight: usize,

  pub all_spawners_binding_order: Vec<usize>,
  pub polling_pipeline: GPUComputePipeline,
  pub resource: TaskGroupExecutorResource,
  pub before_execute: Option<Box<dyn Fn(&mut DeviceParallelComputeCtx, &Self)>>,
  pub after_execute: Option<Box<dyn Fn(&mut DeviceParallelComputeCtx, &Self)>>,
}

pub struct TaskGroupBuildSource {
  pub payload_ty: ShaderSizedValueType,
  pub self_task_idx: usize,
  pub task: OpaqueTask,
  pub max_in_flight: usize,
}

pub(super) struct TaskGroupPreBuild {
  pub shader: ShaderBuildingCtx,
  pub cx: ShaderComputePipelineBuilder,
  pub state_to_resolve: DynamicTypeBuilder,
  pub invocation: Box<dyn ShaderFutureInvocation<Output = Box<dyn Any>>>,
  pub tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstanceLateResolved>,
  pub self_task_idx: usize,
}

impl TaskGroupExecutor {
  pub(super) fn pre_build(
    internal: &TaskGroupBuildSource,
    task_type: usize,
    task_group_shared_info: &mut Vec<(
      TaskGroupDeviceInvocationInstanceLateResolved,
      FastHashSet<usize>,
    )>,
  ) -> TaskGroupPreBuild {
    let mut cx = compute_shader_builder();

    let mut build_ctx = DeviceTaskSystemBuildCtx {
      compute_cx: &mut cx,
      state_builder: DynamicTypeBuilder::new_named(&format!("Task_states_{}", task_type)),
      task_group_shared_info,
      tasks_depend_on_self: Default::default(),
      self_task_idx: task_type,
    };

    let invocation = internal.task.build_poll(&mut build_ctx);
    let state_builder = build_ctx.state_builder;
    let outer_builder = take_build_api();

    TaskGroupPreBuild {
      shader: outer_builder,
      state_to_resolve: state_builder,
      invocation,
      tasks_depend_on_self: build_ctx.tasks_depend_on_self,
      cx,
      self_task_idx: task_type,
    }
  }

  pub(super) fn build(
    mut pre_build: TaskGroupPreBuild,
    task_build_source: &TaskGroupBuildSource,
    pcx: &mut DeviceParallelComputeCtx,
    resources: &[TaskGroupExecutorResource],
    parent_dependencies: &FastHashSet<usize>,
  ) -> TaskGroupExecutor {
    set_build_api(pre_build.shader);

    let mut all_spawners = FastHashMap::default();
    let mut all_spawners_binding_order = Vec::default();

    for (&dep, spawner_to_resolve) in &pre_build.tasks_depend_on_self {
      all_spawners.entry(dep).or_insert_with(|| {
        all_spawners_binding_order.push(dep);
        let spawner = resources[dep].build_shader_for_spawner(&mut pre_build.cx);
        spawner_to_resolve.resolve(spawner.clone());
        spawner
      });
    }
    for &dep in parent_dependencies {
      all_spawners.entry(dep).or_insert_with(|| {
        all_spawners_binding_order.push(dep);
        resources[dep].build_shader_for_spawner(&mut pre_build.cx)
      });
    }

    let mut cx = pre_build.cx;
    let resource = resources[task_build_source.self_task_idx].clone();

    let self_spawner = resource.build_shader_for_spawner(&mut cx);

    let indices = self_spawner.active_task_idx.storage.clone();
    let active_task_count = self_spawner.active_task_idx.current_size.clone();
    let pool = self_spawner.task_pool.clone();

    let active_idx = cx.global_invocation_id().x();

    // even if the task is out of active bound, we still required to poll something to maintain the uniform control flow.
    // the task to poll for this case always resides at index 0 and always stay in pending state.
    let task_index = active_idx
      .less_than(active_task_count.load())
      .select_branched(|| indices.index(active_idx).load(), || val(0));

    let item = pool.rw_states(task_index);
    pre_build.state_to_resolve.resolve(item);

    let mut poll_ctx = DeviceTaskSystemPollCtx {
      self_task_idx: task_index,
      self_task: pool.clone(),
      compute_cx: &mut cx,
      invocation_registry: Default::default(),
      self_task_type_id: pre_build.self_task_idx as u32,
    };

    let poll_result = pre_build.invocation.device_poll(&mut poll_ctx);

    if_by(poll_ctx.is_fallback_task().not(), || {
      if_by(poll_result.is_resolved(), || {
        pool
          .rw_task_state(task_index)
          .store(TASK_STATUE_FLAG_FINISHED);

        let parent_index = pool.rw_parent_task_index(task_index).load();
        let parent_task_type_id = pool.rw_parent_task_type_id(task_index).load();

        if_by(parent_index.equals(u32::MAX), || {
          // if we do not have parent task, then we should cleanup ourself
          self_spawner.cleanup_finished_task_state_and_payload(task_index);
        })
        .else_by(|| {
          let mut switcher = switch_by(parent_task_type_id);
          for dep in parent_dependencies {
            switcher = switcher.case(*dep as u32, || {
              let spawner = all_spawners.get(dep).unwrap();
              spawner.wake_task_dyn(parent_index);
            });
          }
          switcher.end_with_default(|| {});
        });
      })
      .else_by(|| {
        pool
          .rw_task_state(task_index)
          .store(TASK_STATUE_FLAG_GO_TO_SLEEP);
      });
    });

    cx.config_work_group_size(TASK_EXECUTION_WORKGROUP_SIZE);

    let polling_pipeline = cx.create_compute_pipeline(&pcx.gpu.device).unwrap();

    TaskGroupExecutor {
      polling_pipeline,
      max_in_flight: task_build_source.max_in_flight,
      resource,
      state_desc: pre_build.state_to_resolve.meta_info(),
      all_spawners_binding_order,
      before_execute: None,
      after_execute: None,
    }
  }

  pub fn execute(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    all_tasks: &[Self],
    task_source: &TaskGroupBuildSource,
  ) {
    self.prepare_execution(cx);

    if let Some(f) = self.before_execute.as_ref() {
      f(cx, self)
    }

    cx.record_pass(|pass, device| {
      let imp = &mut self.resource;

      let active_execution_size =
        imp
          .active_task_idx
          .prepare_dispatch_size(pass, device, TASK_EXECUTION_WORKGROUP_SIZE);

      // dispatch tasks
      let mut bb = BindingBuilder::default();

      let mut ctx = DeviceTaskSystemBindCtx { binder: &mut bb };

      task_source.task.bind_input(&mut ctx);

      let all_task_group_sources: Vec<_> = all_tasks.iter().map(|t| &t.resource).collect();
      for extra in &self.all_spawners_binding_order {
        all_task_group_sources[*extra].bind_for_spawner(&mut ctx);
      }

      imp.bind_for_spawner(&mut ctx);
      bb.setup_compute_pass(pass, device, &self.polling_pipeline);
      pass.dispatch_workgroups_indirect_by_buffer_resource_view(&active_execution_size);
    });

    self.compact_alive_tasks(cx);

    if let Some(f) = self.after_execute.as_ref() {
      f(cx, self)
    }
  }

  fn compact_alive_tasks(&mut self, ctx: &mut DeviceParallelComputeCtx) {
    // this is required because the task may spawn it self
    ctx.record_pass(|pass, device| {
      let imp = &self.resource;
      imp.active_task_idx.commit_size(pass, device, true);
    });

    let imp = &mut self.resource;
    // compact active task buffer
    let active_tasks =
      ParallelComputeFromAbstractStorageBuffer(imp.active_task_idx.storage.clone());

    // the input and output shares one single binding so it can be aliased
    let active_task_idx_back_buffer = imp.active_task_idx_back_buffer.get_gpu_buffer_view();
    let active_tasks_back_buffer =
      StorageBufferDataView::try_from_raw(active_task_idx_back_buffer).unwrap();

    let re = active_tasks
      .clone()
      .stream_compaction(ActiveTaskCompact {
        active_size: imp.active_task_idx.current_size.clone(),
        active_tasks: imp.active_task_idx.storage.clone(),
        task_pool: imp.task_pool.clone(),
      })
      .materialize_storage_buffer_into(active_tasks_back_buffer, ctx);

    std::mem::swap(
      &mut imp.active_task_idx.storage,
      &mut imp.active_task_idx_back_buffer,
    );

    let new_active_task_size = re.size.unwrap();

    ctx.record_pass(|pass, device| {
      // manually update the alive task bumper's current size
      let imp = &mut self.resource;
      let hasher = shader_hasher_from_marker_ty!(SizeUpdate);
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder.config_work_group_size(1);
        let new_size = builder.bind_by(&new_active_task_size);
        let current_size = builder.bind_abstract_storage(&imp.active_task_idx.current_size);
        current_size.store(new_size.load().x());
        builder
      });

      BindingBuilder::default()
        .with_bind(&new_active_task_size)
        .with_bind_abstract_storage(&imp.active_task_idx.current_size)
        .setup_compute_pass(pass, device, &pipeline);

      pass.dispatch_workgroups(1, 1, 1);
    });
  }

  pub fn prepare_execution_and_compact_living_task(&mut self, ctx: &mut DeviceParallelComputeCtx) {
    self.prepare_execution(ctx);
    self.compact_alive_tasks(ctx);
  }

  pub fn prepare_execution(&mut self, ctx: &mut DeviceParallelComputeCtx) {
    // commit bumpers
    ctx.record_pass(|pass, device| {
      let imp = &self.resource;
      imp.active_task_idx.commit_size(pass, device, true);
      imp.empty_index_pool.commit_size(pass, device, false);
      imp.new_removed_task_idx.commit_size(pass, device, true);
    });

    ctx.record_pass(|pass, device| {
      let imp = &self.resource;
      // drain empty to empty pool
      imp
        .new_removed_task_idx
        .drain_self_into_the_other(&imp.empty_index_pool, pass, device);
    });
  }
}

#[derive(Clone)]
pub struct TaskGroupExecutorResource {
  /// reused as active task compaction target
  pub active_task_idx_back_buffer: BoxedAbstractStorageBuffer<[u32]>,
  pub active_task_idx: DeviceBumpAllocationInstance<u32>,
  pub new_removed_task_idx: DeviceBumpAllocationInstance<u32>,
  pub empty_index_pool: DeviceBumpAllocationInstance<u32>,
  pub task_pool: TaskPool,
  pub size: usize,
  payload_ty: ShaderSizedValueType,
  index: usize,
}

impl TaskGroupExecutorResource {
  /// should call init before actually use this
  pub fn create_with_size(
    index: usize,
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    payload_ty: ShaderSizedValueType,
    cx: &mut DeviceParallelComputeCtx,
    allocator: &MaybeCombinedStorageAllocator,
    a_a: &MaybeCombinedAtomicU32StorageAllocator,
  ) -> Self {
    let device = &cx.gpu.device;
    Self {
      active_task_idx_back_buffer: allocator.allocate((size * 4) as u64, device),
      active_task_idx: DeviceBumpAllocationInstance::new(size, device, allocator, a_a),
      new_removed_task_idx: DeviceBumpAllocationInstance::new(size, device, allocator, a_a),
      empty_index_pool: DeviceBumpAllocationInstance::new(size, device, allocator, a_a),
      // add one is for the first default task
      task_pool: TaskPool::create_with_size(
        index,
        size + 1,
        state_desc,
        payload_ty.clone(),
        device,
        allocator,
      ),
      size,
      payload_ty,
      index,
    }
  }

  pub fn init(&self, cx: &mut DeviceParallelComputeCtx) {
    // fill the empty pool, allocate the first default task
    cx.record_pass(|pass, device| {
      let hasher = shader_hasher_from_marker_ty!(PrepareEmptyIndices)
        .with_hash(self.index)
        .with_hash(&self.payload_ty);

      let workgroup_size = 256;
      let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder.config_work_group_size(workgroup_size);

        let empty_pool = builder.bind_abstract_storage(&self.empty_index_pool.storage);
        let empty_pool_size = builder.bind_abstract_storage(&self.empty_index_pool.current_size);
        let task_pool = self.task_pool.build_shader(&mut builder);
        let id = builder.global_invocation_id().x();

        if_by(id.equals(0), || {
          empty_pool_size.store(empty_pool.array_length());
          task_pool.spawn_new_task_dyn(
            val(0),
            ShaderNodeExpr::Zeroed {
              target: self.payload_ty.clone(),
            }
            .insert_api_raw(),
            TaskParentRef::none_parent(),
            &self.payload_ty,
          );
        });

        if_by(id.less_than(empty_pool.array_length()), || {
          empty_pool.index(id).store(id + val(1));
        });

        builder
      });

      let mut builder = BindingBuilder::default()
        .with_bind_abstract_storage(&self.empty_index_pool.storage)
        .with_bind_abstract_storage(&self.empty_index_pool.current_size);

      self.task_pool.bind(&mut builder);

      builder.setup_compute_pass(pass, device, &pipeline);

      pass.dispatch_workgroups(
        compute_dispatch_size(self.size as u32, workgroup_size),
        1,
        1,
      );
    });
  }

  pub fn build_shader_for_spawner(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> TaskGroupDeviceInvocationInstance {
    TaskGroupDeviceInvocationInstance {
      new_removed_task_idx: self.new_removed_task_idx.build_allocator_shader(cx),
      empty_index_pool: self.empty_index_pool.build_deallocator_shader(cx),
      task_pool: self.task_pool.build_shader(cx),
      active_task_idx: self.active_task_idx.build_allocator_shader(cx),
    }
  }

  pub fn bind_for_spawner(&self, cx: &mut BindingBuilder) {
    self.new_removed_task_idx.bind_allocator(cx);
    self.empty_index_pool.bind_allocator(cx);
    self.task_pool.bind(cx);
    self.active_task_idx.bind_allocator(cx);
  }
}

#[derive(Clone)]
pub struct TaskGroupDeviceInvocationInstance {
  new_removed_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  empty_index_pool: DeviceBumpDeAllocationInvocationInstance<u32>,
  active_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  task_pool: TaskPoolInvocationInstance,
}

impl TaskGroupDeviceInvocationInstance {
  #[must_use]
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(
    &self,
    payload: Node<T>,
    parent_ref: TaskParentRef,
  ) -> Option<TaskFutureInvocationRightValue> {
    self.spawn_new_task_dyn(payload.handle(), parent_ref, &T::sized_ty())
  }

  #[must_use]
  pub fn spawn_new_task_dyn(
    &self,
    payload: ShaderNodeRawHandle,
    parent_ref: TaskParentRef,
    ty: &ShaderSizedValueType,
  ) -> Option<TaskFutureInvocationRightValue> {
    let (idx, success) = self.empty_index_pool.bump_deallocate(); // todo, error report
    shader_assert(success);

    self
      .task_pool
      .spawn_new_task_dyn(idx, payload, parent_ref, ty);
    let (_, success) = self.active_task_idx.bump_allocate(idx); // todo, error report
    shader_assert(success);

    Some(TaskFutureInvocationRightValue { task_handle: idx })
  }

  pub fn wake_task_dyn(&self, task_id: Node<u32>) {
    let is_in_active_list = self
      .task_pool
      .rw_task_state(task_id)
      .load()
      .equals(TASK_STATUE_FLAG_GO_TO_SLEEP);
    self
      .task_pool
      .rw_task_state(task_id)
      .store(TASK_STATUE_FLAG_NOT_FINISHED_WAKEN);
    if_by(is_in_active_list.not(), || {
      let (_, success) = self.active_task_idx.bump_allocate(task_id); // todo, error report
      shader_assert(success);
    });
  }

  #[must_use]
  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    self.poll_task_dyn(task_id, |x| {
      argument_read_back(T::create_view_from_raw_ptr(x).load())
    })
  }

  #[must_use]
  pub fn poll_task_dyn(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(BoxedShaderPtr) + Copy,
  ) -> Node<bool> {
    let finished = self.is_task_finished(task_id);
    if_by(finished, || {
      argument_read_back(self.rw_payload_dyn(task_id));
      self.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }

  fn cleanup_finished_task_state_and_payload(&self, task: Node<u32>) {
    let (_, success) = self.new_removed_task_idx.bump_allocate(task);
    shader_assert(success);
    self
      .task_pool
      .rw_task_state(task)
      .store(TASK_STATUE_FLAG_TASK_NOT_EXIST);
    // todo consider zeroing the state and payload
  }

  fn is_task_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self.task_pool.is_task_finished(task_id)
  }

  pub fn read_back_payload<T: ShaderSizedValueNodeType>(&self, task_id: Node<u32>) -> Node<T> {
    self.task_pool.rw_payload::<T>(task_id).load()
  }
  fn rw_payload_dyn(&self, task_id: Node<u32>) -> BoxedShaderPtr {
    self.task_pool.rw_payload_dyn(task_id)
  }
}
