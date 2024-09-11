use crate::*;

mod task_pool;
use task_pool::*;

mod dispatch_compact;
use dispatch_compact::*;

mod task_group;
pub use task_group::*;

mod future_context;
pub use future_context::*;

pub const TASK_EXECUTION_WORKGROUP_SIZE: u32 = 128;

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
  pub fn new(current_prepared_execution_size: usize, max_recursion_depth: usize) -> Self {
    Self {
      task_groups: Default::default(),
      max_recursion_depth,
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
    self.define_task_dyn(
      Box::new(OpaqueTaskWrapper(future)) as OpaqueTask,
      P::sized_ty(),
      device,
      pass,
    )
  }

  #[inline(never)]
  pub fn define_task_dyn(
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
      all_task_group_sources: task_group_sources,
      tasks_depend_on_self: Default::default(),
    };

    let state = task.build_poll(&mut build_ctx);

    let state_desc = build_ctx.state_builder.meta_info();
    let tasks_depend_on_self = build_ctx.tasks_depend_on_self.keys().cloned().collect();

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
    let active_task_count = cx.bind_by(&resource.alive_task_idx.current_size);
    let pool = resource.task_pool.build_shader(&mut cx);

    let active_idx = cx.global_invocation_id().x();
    if_by(active_idx.less_than(active_task_count.load()), || {
      let task_index = indices.index(active_idx).load();

      let item = pool.rw_states(task_index);
      state_builder.resolve(item.cast_untyped_node());

      let mut poll_ctx = DeviceTaskSystemPollCtx {
        self_task_idx: task_index,
        self_task: pool.clone(),
        compute_cx: &mut cx,
        invocation_registry: Default::default(),
      };

      let poll_result = state.device_poll(&mut poll_ctx);
      if_by(poll_result.is_ready, || {
        pool
          .rw_is_finished(task_index)
          .store(TASK_STATUE_FLAG_FINISHED);
      });
    });

    cx.config_work_group_size(TASK_EXECUTION_WORKGROUP_SIZE);

    let polling_pipeline = cx.create_compute_pipeline(device).unwrap();

    let task_executor = TaskGroupExecutor {
      polling_pipeline,
      resource,
      state_desc,
      task_type_desc,
      tasks_depend_on_self,
      required_poll_count: task.required_poll_count(),
      task,
      self_task_idx: task_type,
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
      s.reset(ctx, dispatch_size);
      ctx.record_pass(|pass, _| s.resize(gpu, dispatch_size, self.max_recursion_depth, pass))
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

    let dispatch_size_buffer = create_gpu_readonly_storage(&dispatch_size, device);

    let hasher = PipelineHasher::default().with_hash(task_spawner.type_id());
    let workgroup_size = 256;
    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
      builder.config_work_group_size(workgroup_size);

      let dispatch_size = builder.bind_by(&dispatch_size_buffer);
      let instance = task_group.resource.build_shader_for_spawner(&mut builder);
      let id = builder.global_invocation_id().x();

      if_by(id.less_than(dispatch_size.load()), || {
        let payload = task_spawner(id);
        instance
          .spawn_new_task(payload)
          .expect("payload miss match");
      });

      builder
    });

    let mut bb = BindingBuilder::new_as_compute().with_bind(&dispatch_size_buffer);
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

  pub async fn read_back_execution_states(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> TaskGraphExecutionStates {
    self.task_groups.iter_mut().for_each(|task| {
      task.prepare_execution(cx);
    });
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

  // todo, dispatch should in reverse order
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
