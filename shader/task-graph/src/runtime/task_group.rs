use super::*;

pub type OpaqueTask = Box<
  dyn ShaderFuture<
    Output = Box<dyn Any>,
    Invocation = Box<dyn ShaderFutureInvocation<Output = Box<dyn Any>>>,
  >,
>;

pub struct TaskGroupExecutor {
  pub internal: TaskGroupExecutorInternal,
  pub state_desc: DynamicTypeMetaInfo,

  pub all_spawners_binding_order: Vec<usize>,
  pub polling_pipeline: GPUComputePipeline,
  pub resource: TaskGroupExecutorResource,
}

pub struct TaskGroupExecutorInternal {
  pub payload_ty: ShaderSizedValueType,
  pub self_task_idx: usize,
  pub required_poll_count: usize,
  pub task: OpaqueTask,
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
    internal: &TaskGroupExecutorInternal,
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
    task_build_source: TaskGroupExecutorInternal,
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

    let indices = self_spawner.active_task_idx.storage;
    let active_task_count = self_spawner.active_task_idx.current_size;
    let pool = self_spawner.task_pool.clone();

    let active_idx = cx.global_invocation_id().x();
    if_by(active_idx.less_than(active_task_count.load()), || {
      let task_index = indices.index(active_idx).load();

      let item = pool.rw_states(task_index);
      pre_build.state_to_resolve.resolve(item.cast_untyped_node());

      let mut poll_ctx = DeviceTaskSystemPollCtx {
        self_task_idx: task_index,
        self_task: pool.clone(),
        compute_cx: &mut cx,
        invocation_registry: Default::default(),
        self_task_type_id: pre_build.self_task_idx as u32,
      };

      let poll_result = pre_build.invocation.device_poll(&mut poll_ctx);
      if_by(poll_result.is_ready, || {
        pool
          .rw_is_finished(task_index)
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
          .rw_is_finished(task_index)
          .store(TASK_STATUE_FLAG_NOT_FINISHED_SLEEP);
      });
    });

    cx.config_work_group_size(TASK_EXECUTION_WORKGROUP_SIZE);

    let polling_pipeline = cx.create_compute_pipeline(&pcx.gpu.device).unwrap();

    TaskGroupExecutor {
      polling_pipeline,
      resource,
      internal: task_build_source,
      state_desc: pre_build.state_to_resolve.meta_info(),
      all_spawners_binding_order,
    }
  }

  pub fn execute(&mut self, cx: &mut DeviceParallelComputeCtx, all_tasks: &[Self]) {
    self.prepare_execution(cx);

    cx.record_pass(|pass, device| {
      let imp = &mut self.resource;

      let active_execution_size =
        imp
          .active_task_idx
          .prepare_dispatch_size(pass, device, TASK_EXECUTION_WORKGROUP_SIZE);

      // dispatch tasks
      let mut bb = BindingBuilder::new_as_compute();

      let mut ctx = DeviceTaskSystemBindCtx { binder: &mut bb };

      self.internal.task.bind_input(&mut ctx);

      let all_task_group_sources: Vec<_> = all_tasks.iter().map(|t| &t.resource).collect();
      for extra in &self.all_spawners_binding_order {
        all_task_group_sources[*extra].bind_for_spawner(&mut ctx);
      }

      imp.bind_for_spawner(&mut ctx);

      bb.setup_compute_pass(pass, device, &self.polling_pipeline);
      pass.dispatch_workgroups_indirect_by_buffer_resource_view(&active_execution_size);
    });

    // todo, this must be improved. extra prepare_execution is costly.
    // this is required because when task poll sleep, if we not do alive task compact, when the
    // subsequent task wake the parent in this task group, it will create duplicate invocation.
    //
    // we can not simply clear the alive list because the task could self spawn new tasks.
    // maybe one solution is to add anther task state to mark the task is sleeping but still in alive task.
    // the prepare execution will still compact by this flag(and will reset it), but when child task wake parent,
    //  if it see this special flag the alive task index spawn will be skipped.
    self.prepare_execution(cx);
  }

  pub fn prepare_execution(&mut self, ctx: &mut DeviceParallelComputeCtx) {
    // commit bumpers
    ctx.record_pass(|pass, device| {
      let imp = &mut self.resource;
      imp.active_task_idx.commit_size(pass, device, true);
      imp.empty_index_pool.commit_size(pass, device, false);
      imp.new_removed_task_idx.commit_size(pass, device, true);
    });

    let imp = &mut self.resource;
    // compact active task buffer
    let active_tasks = imp.active_task_idx.storage.clone().into_readonly_view();
    let re = active_tasks
      .clone()
      .stream_compaction(ActiveTaskCompact {
        active_size: imp.active_task_idx.current_size.clone(),
        active_tasks: active_tasks.clone(),
        task_pool: imp.task_pool.clone(),
      })
      .materialize_storage_buffer(ctx);
    imp.active_task_idx.storage = re.buffer.into_rw_view();
    let new_active_task_size = re.size.unwrap();

    ctx.record_pass(|pass, device| {
      // manually update the alive task bumper's current size
      let imp = &mut self.resource;
      let hasher = shader_hasher_from_marker_ty!(SizeUpdate);
      let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
        builder.config_work_group_size(1);
        let new_size = builder.bind_by(&new_active_task_size);
        let current_size = builder.bind_by(&imp.active_task_idx.current_size);
        current_size.store(new_size.load().x());
        builder
      });

      BindingBuilder::new_as_compute()
        .with_bind(&new_active_task_size)
        .with_bind(&imp.active_task_idx.current_size)
        .setup_compute_pass(pass, device, &pipeline);

      pass.dispatch_workgroups(1, 1, 1);

      // drain empty to empty pool
      imp
        .new_removed_task_idx
        .drain_self_into_the_other(&imp.empty_index_pool, pass, device);
    });
  }
}

#[derive(Clone)]
pub struct TaskGroupExecutorResource {
  pub active_task_idx: DeviceBumpAllocationInstance<u32>,
  pub new_removed_task_idx: DeviceBumpAllocationInstance<u32>,
  pub empty_index_pool: DeviceBumpAllocationInstance<u32>,
  pub task_pool: TaskPool,
  pub size: usize,
}

impl TaskGroupExecutorResource {
  pub fn create_with_size(
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    payload_ty: ShaderSizedValueType,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Self {
    let device = &cx.gpu.device;
    // to support self spawning, some buffer's size is doubled for max extra allocation space
    let res = Self {
      active_task_idx: DeviceBumpAllocationInstance::new(size * 2, device),
      new_removed_task_idx: DeviceBumpAllocationInstance::new(size, device),
      empty_index_pool: DeviceBumpAllocationInstance::new(size * 2, device),
      task_pool: TaskPool::create_with_size(size * 2, state_desc, payload_ty, device),
      size,
    };

    cx.record_pass(|pass, device| {
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

        if_by(id.less_than(empty_pool.array_length()), || {
          empty_pool.index(id).store(id);
        });
        builder
      });

      BindingBuilder::new_as_compute()
        .with_bind(&res.empty_index_pool.storage)
        .with_bind(&res.empty_index_pool.current_size)
        .setup_compute_pass(pass, device, &pipeline);

      pass.dispatch_workgroups(
        compute_dispatch_size((size * 2) as u32, workgroup_size),
        1,
        1,
      );
    });

    res
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
    self.spawn_new_task_dyn(payload.cast_untyped_node(), parent_ref, &T::sized_ty())
  }

  #[must_use]
  pub fn spawn_new_task_dyn(
    &self,
    payload: Node<AnyType>,
    parent_ref: TaskParentRef,
    ty: &ShaderSizedValueType,
  ) -> Option<TaskFutureInvocationRightValue> {
    let (idx, success) = self.empty_index_pool.bump_deallocate();

    if_by(success, || {
      self
        .task_pool
        .spawn_new_task_dyn(idx, payload, parent_ref, ty);
      let (_, success) = self.active_task_idx.bump_allocate(idx); // todo, error report
      if_by(success.not(), || loop_by(|_| {}));
    })
    .else_by(|| {
      loop_by(|_| {})
      // error report, theoretically unreachable
    });

    Some(TaskFutureInvocationRightValue { task_handle: idx })
  }

  pub fn wake_task_dyn(&self, task_id: Node<u32>) {
    self
      .task_pool
      .rw_is_finished(task_id)
      .store(TASK_STATUE_FLAG_NOT_FINISHED_WAKEN);
    let (_, success) = self.active_task_idx.bump_allocate(task_id); // todo, error report
    if_by(success.not(), || loop_by(|_| {}));
  }

  #[must_use]
  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    self.poll_task_dyn(task_id, |x| unsafe {
      argument_read_back(x.cast_type::<ShaderStoragePtr<T>>().load())
    })
  }

  #[must_use]
  pub fn poll_task_dyn(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(StorageNode<AnyType>) + Copy,
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
    self
      .task_pool
      .rw_is_finished(task)
      .store(TASK_STATUE_FLAG_TASK_NOT_EXIST);
    // todo consider zeroing the state and payload
    if_by(success.not(), || {
      loop_by(|_| {})
      // error report, theoretically unreachable
    });
  }

  fn is_task_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self.task_pool.is_task_finished(task_id)
  }

  pub fn read_back_payload<T: ShaderSizedValueNodeType>(&self, task_id: Node<u32>) -> Node<T> {
    self.task_pool.rw_payload(task_id).load()
  }
  fn rw_payload_dyn(&self, task_id: Node<u32>) -> StorageNode<AnyType> {
    self.task_pool.rw_payload_dyn(task_id)
  }
}
