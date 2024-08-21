use super::*;

pub type OpaqueTask = Box<
  dyn DeviceFuture<
    Output = Box<dyn Any>,
    Invocation = Box<dyn DeviceFutureInvocation<Output = Box<dyn Any>>>,
  >,
>;

pub struct TaskGroupExecutor {
  pub state_desc: DynamicTypeMetaInfo,
  pub task_type_desc: ShaderStructMetaInfo,
  pub task: OpaqueTask,

  pub polling_pipeline: GPUComputePipeline,
  pub tasks_depend_on_self: Vec<usize>,
  pub resource: TaskGroupExecutorResource,
  pub required_poll_count: usize,
}

impl TaskGroupExecutor {
  pub fn reset(&mut self, ctx: &mut DeviceParallelComputeCtx, dispatch_size: usize) {
    self.task.reset(ctx, dispatch_size as u32);
  }

  pub fn execute(&mut self, cx: &mut DeviceParallelComputeCtx, all_tasks: &[Self]) {
    // update bumpers' size
    cx.record_pass(|pass, device| {
      let imp = &mut self.resource;
      imp.alive_task_idx.commit_size(pass, device, true);
      imp.empty_index_pool.commit_size(pass, device, false);
      imp.new_removed_task_idx.commit_size(pass, device, true);
    });

    {
      let imp = &mut self.resource;
      // compact active task buffer
      let alive_tasks = imp.alive_task_idx.storage.clone().into_readonly_view();
      let size = imp.alive_task_idx.current_size.clone();
      let re = alive_tasks
        .clone()
        .stream_compaction(ActiveTaskCompact {
          alive_size: size,
          active_tasks: alive_tasks.clone(),
          task_pool: imp.task_pool.clone(),
        })
        .materialize_storage_buffer(cx);
      imp.alive_task_idx.storage = re.buffer.into_rw_view();
      let new_size = re.size.unwrap();

      // this is not good, but it's ok for now
      cx.record_pass(|pass, device| {
        let imp = &mut self.resource;
        let hasher = shader_hasher_from_marker_ty!(SizeUpdate);
        let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
          builder.config_work_group_size(1);
          let new_size = builder.bind_by(&new_size);
          let current_size = builder.bind_by(&imp.alive_task_idx.current_size);
          current_size.store(new_size.load().x());
          builder
        });

        BindingBuilder::new_as_compute()
          .with_bind(&new_size)
          .with_bind(&imp.alive_task_idx.current_size)
          .setup_compute_pass(pass, device, &pipeline);

        pass.dispatch_workgroups(1, 1, 1);
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
      pass.dispatch_workgroups_indirect_by_buffer_resource_view(&size);
    });
  }

  pub fn resize(
    &mut self,
    gpu: &GPU,
    size: usize,
    max_recursion_depth: usize,
    pass: &mut GPUComputePass,
  ) {
    let required_size = size * max_recursion_depth;
    if self.resource.size != required_size {
      self.resource = TaskGroupExecutorResource::create_with_size(
        required_size,
        self.state_desc.clone(),
        self.task_type_desc.clone(),
        &gpu.device,
        pass,
      );
    }
  }
}

pub struct TaskGroupExecutorResource {
  pub alive_task_idx: DeviceBumpAllocationInstance<u32>,
  // main_dispatch_size: StorageBufferDataView<DispatchIndirectArgs>,
  pub new_removed_task_idx: DeviceBumpAllocationInstance<u32>,
  pub empty_index_pool: DeviceBumpAllocationInstance<u32>,
  pub task_pool: TaskPool,
  pub size: usize,
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

      if_by(id.less_than(empty_pool.array_length()), || {
        empty_pool.index(id).store(id);
      });
      builder
    });

    BindingBuilder::new_as_compute()
      .with_bind(&res.empty_index_pool.storage)
      .with_bind(&res.empty_index_pool.current_size)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(compute_dispatch_size(size as u32 * 2, workgroup_size), 1, 1);

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
      alive_task_idx: self.alive_task_idx.build_allocator_shader(cx),
    }
  }

  pub fn bind_for_spawner(&self, cx: &mut BindingBuilder) {
    self.new_removed_task_idx.bind_allocator(cx);
    self.empty_index_pool.bind_allocator(cx);
    self.task_pool.bind(cx);
    self.alive_task_idx.bind_allocator(cx);
  }
}

pub struct TaskGroupDeviceInvocationInstance {
  new_removed_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  empty_index_pool: DeviceBumpDeAllocationInvocationInstance<u32>,
  alive_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  task_pool: TaskPoolInvocationInstance,
}

impl TaskGroupDeviceInvocationInstance {
  #[must_use]
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(&self, payload: Node<T>) -> Option<Node<u32>> {
    self.spawn_new_task_dyn(payload.cast_untyped_node(), T::sized_ty())
  }

  #[must_use]
  pub fn spawn_new_task_dyn(
    &self,
    payload: Node<AnyType>,
    ty: ShaderSizedValueType,
  ) -> Option<Node<u32>> {
    let (idx, success) = self.empty_index_pool.bump_deallocate();
    if_by(success, || {
      self.task_pool.spawn_new_task_dyn(idx, payload, ty);
      self.alive_task_idx.bump_allocate(idx);
    })
    .else_by(|| {
      // error report, theoretically unreachable
    });
    Some(idx)
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

  pub fn read_back_payload<T: ShaderSizedValueNodeType>(&self, task_id: Node<u32>) -> Node<T> {
    self.task_pool.rw_payload(task_id).load()
  }
}
