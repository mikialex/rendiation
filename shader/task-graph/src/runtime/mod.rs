use crate::*;

mod task_pool;
pub use task_pool::TaskParentRef;
use task_pool::*;

mod dispatch_compact;
use dispatch_compact::*;

mod task_group;
pub use task_group::*;

mod future_context;
pub use future_context::*;

pub const TASK_EXECUTION_WORKGROUP_SIZE: u32 = 128;

#[derive(Clone, Debug)]
pub struct TaskGraphExecutionStates {
  pub wake_counts: Vec<u32>,
  pub sleep_or_finished_counts: Vec<u32>,
  pub empty_counts: Vec<u32>,
}

impl TaskGraphExecutionStates {
  pub fn is_empty(&self) -> bool {
    self.sleep_or_finished_counts.iter().all(|c| *c == 0)
      && self.wake_counts.iter().all(|c| *c == 0)
  }
}

#[derive(Clone, Debug)]
pub struct TaskGraphExecutionDebugInfo {
  pub info: Vec<TaskExecutionDebugInfo>,
}

#[derive(Clone)]
pub struct TaskExecutionDebugInfo {
  pub active_idx: Vec<u32>,
  pub empty_idx: Vec<u32>,
  pub task_states: Vec<u8>,
}

impl Debug for TaskExecutionDebugInfo {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TaskExecutionDebugInfo")
      .field("active_idx", &self.active_idx)
      .field("empty_idx", &self.empty_idx)
      .finish() // skip task_states
  }
}

#[derive(Default)]
pub struct DeviceTaskGraphBuildSource {
  tasks: Vec<TaskGroupBuildSource>,
  pub capacity: usize,
}

impl DeviceTaskGraphBuildSource {
  pub fn define_task<P, F>(&mut self, future: F, max_in_flight: usize) -> u32
  where
    F: ShaderFuture<Output = ()> + 'static,
    P: ShaderSizedValueNodeType,
  {
    self.define_task_dyn(
      Box::new(OpaqueTaskWrapper(future)) as OpaqueTask,
      P::sized_ty(),
      max_in_flight,
    )
  }
  pub fn next_task_idx(&self) -> u32 {
    self.tasks.len() as u32
  }

  #[inline(never)]
  pub fn define_task_dyn(
    &mut self,
    task: OpaqueTask,
    payload_ty: ShaderSizedValueType,
    max_in_flight: usize,
  ) -> u32 {
    let task_type = self.tasks.len();

    self.tasks.push(TaskGroupBuildSource {
      payload_ty,
      self_task_idx: task_type,
      task,
      max_in_flight,
    });

    task_type as u32
  }

  pub fn build(&self, cx: &mut DeviceParallelComputeCtx) -> DeviceTaskGraphExecutor {
    let mut task_group_shared_info = Vec::new();
    for _ in 0..self.tasks.len() {
      task_group_shared_info.push((Default::default(), Default::default()));
    }

    // let enable_buffer_combine = true;

    let mut pre_builds = Vec::new();
    let mut task_group_sources = Vec::new();
    let buffer_allocator =
      MaybeCombinedStorageAllocator::new(&cx.gpu, "task graph execution resources", true, true);
    let atomic_allocator = MaybeCombinedAtomicU32StorageAllocator::new(
      &cx.gpu,
      "task graph execution atomic resources",
      true,
    );

    for (i, task_build_source) in self.tasks.iter().enumerate() {
      let pre_build =
        TaskGroupExecutor::pre_build(task_build_source, i, &mut task_group_shared_info);

      let init_size = task_build_source.max_in_flight * self.capacity;
      let resource = TaskGroupExecutorResource::create_with_size(
        i,
        init_size,
        pre_build.state_to_resolve.meta_info(),
        task_build_source.payload_ty.clone(),
        cx,
        &buffer_allocator,
        &atomic_allocator,
      );

      task_group_sources.push(resource);
      pre_builds.push(pre_build);
    }

    buffer_allocator.rebuild();
    atomic_allocator.rebuild();

    for res in &task_group_sources {
      res.init(cx);
    }

    let mut task_group_executors = Vec::new();

    for ((task_build_source, pre_build), (_, parent_dependencies)) in self
      .tasks
      .iter()
      .zip(pre_builds)
      .zip(&task_group_shared_info)
    {
      let exe = TaskGroupExecutor::build(
        pre_build,
        task_build_source,
        cx,
        &task_group_sources,
        parent_dependencies,
      );
      task_group_executors.push(exe);
    }

    DeviceTaskGraphExecutor {
      task_groups: task_group_executors,
      current_prepared_execution_size: self.capacity,
    }
  }
}

pub struct DeviceTaskGraphExecutor {
  task_groups: Vec<TaskGroupExecutor>,
  current_prepared_execution_size: usize,
}

pub trait TaskSpawnerInvocation<T> {
  fn spawn_task(&self, global_id: Node<u32>, count: Node<u32>) -> Node<T>;
}

pub trait TaskSpawner<T>: ShaderHashProvider {
  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn TaskSpawnerInvocation<T>>;
  fn bind(&self, cx: &mut BindingBuilder);
}

impl DeviceTaskGraphExecutor {
  pub fn set_task_before_execution_hook(
    &mut self,
    task_id: usize,
    hook: impl Fn(&mut DeviceParallelComputeCtx, &TaskGroupExecutor) + 'static,
  ) {
    self.task_groups[task_id].before_execute = Some(Box::new(hook));
  }
  pub fn set_task_after_execution_hook(
    &mut self,
    task_id: usize,
    hook: impl Fn(&mut DeviceParallelComputeCtx, &TaskGroupExecutor) + 'static,
  ) {
    self.task_groups[task_id].after_execute = Some(Box::new(hook));
  }

  /// Allocate task directly in the task pool by dispatching compute shader.
  ///
  /// The task_spawner should not has any shader variant.
  ///
  /// T must match given task_id's payload type
  pub fn dispatch_allocate_init_task_by_fn<T: ShaderSizedValueNodeType>(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    task_count: u32,
    task_id: u32,
    task_spawner: impl FnOnce(Node<u32>) -> Node<T> + Copy + 'static,
  ) {
    struct SimpleFnSpawner<T>(T);
    impl<T: 'static> ShaderHashProvider for SimpleFnSpawner<T> {
      fn hash_type_info(&self, hasher: &mut PipelineHasher) {
        self.0.type_id().hash(hasher)
      }
    }
    impl<P, T> TaskSpawner<P> for SimpleFnSpawner<T>
    where
      T: FnOnce(Node<u32>) -> Node<P> + Copy + 'static,
    {
      fn build_invocation(
        &self,
        _: &mut ShaderBindGroupBuilder,
      ) -> Box<dyn TaskSpawnerInvocation<P>> {
        Box::new(SimplerFnSpawnerInvocation(self.0))
      }

      fn bind(&self, _: &mut BindingBuilder) {}
    }
    struct SimplerFnSpawnerInvocation<T>(T);
    impl<P, T> TaskSpawnerInvocation<P> for SimplerFnSpawnerInvocation<T>
    where
      T: FnOnce(Node<u32>) -> Node<P> + Copy,
    {
      fn spawn_task(&self, global_id: Node<u32>, _: Node<u32>) -> Node<P> {
        (self.0)(global_id)
      }
    }

    self.dispatch_allocate_init_task(cx, task_count, task_id, &SimpleFnSpawner(task_spawner));
  }

  /// Allocate task directly in the task pool by dispatching compute shader.
  ///
  /// T must match given task_id's payload type
  pub fn dispatch_allocate_init_task<T: ShaderSizedValueNodeType>(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    task_count: u32,
    task_id: u32,
    task_spawner: &dyn TaskSpawner<T>,
  ) {
    let device = &cx.gpu.device;
    let task_group = &self.task_groups[task_id as usize];

    let dispatch_size_buffer = create_gpu_readonly_storage(&task_count, device);

    let mut hasher = PipelineHasher::default();
    task_spawner.hash_pipeline_with_type_info(&mut hasher);
    let workgroup_size = 256;
    let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
      builder.config_work_group_size(workgroup_size);

      let dispatch_size = builder.bind_by(&dispatch_size_buffer);
      let instance = task_group.resource.build_shader_for_spawner(&mut builder);
      let id = builder.global_invocation_id().x();
      let spawner = task_spawner.build_invocation(builder.bindgroups());

      let dispatch_size = dispatch_size.load();
      if_by(id.less_than(dispatch_size), || {
        let payload = spawner.spawn_task(id, dispatch_size);
        instance
          .spawn_new_task_dyn(
            payload.handle(),
            TaskParentRef::none_parent(),
            &T::sized_ty(),
          )
          .expect("payload miss match");
      });

      builder
    });

    cx.record_pass(|pass, device| {
      let mut bb = BindingBuilder::default().with_bind(&dispatch_size_buffer);
      task_group.resource.bind_for_spawner(&mut bb);
      task_spawner.bind(&mut bb);
      bb.setup_compute_pass(pass, device, &pipeline);

      let size = compute_dispatch_size(task_count, workgroup_size);
      pass.dispatch_workgroups(size, 1, 1);

      task_group
        .resource
        .active_task_idx
        .commit_size(pass, device, true);
      task_group
        .resource
        .empty_index_pool
        .commit_size(pass, device, false);
    })
  }

  pub async fn read_back_execution_states(
    &mut self,
    cx: &mut DeviceParallelComputeCtx<'_>,
  ) -> TaskGraphExecutionStates {
    self.task_groups.iter_mut().for_each(|task| {
      task.prepare_execution_and_compact_living_task(cx);
    });
    cx.flush_pass();

    let result_size = self.task_groups.len() * 4;
    let result_size = NonZeroU64::new(result_size as u64).unwrap();
    let wake_task_counts = create_gpu_read_write_storage::<[u32]>(
      StorageBufferInit::Zeroed(result_size),
      &cx.gpu.device,
    );
    let empty_task_counts = create_gpu_read_write_storage::<[u32]>(
      StorageBufferInit::Zeroed(result_size),
      &cx.gpu.device,
    );

    for (i, task) in self.task_groups.iter().enumerate() {
      let src = task
        .resource
        .active_task_idx
        .current_size
        .get_gpu_buffer_view();
      cx.encoder.copy_buffer_to_buffer(
        src.buffer.gpu(),
        src.range.offset,
        wake_task_counts.buffer.gpu(),
        i as u64 * 4,
        4,
      );
      let src = task
        .resource
        .empty_index_pool
        .current_size
        .get_gpu_buffer_view();
      cx.encoder.copy_buffer_to_buffer(
        src.buffer.gpu(),
        src.range.offset,
        empty_task_counts.buffer.gpu(),
        i as u64 * 4,
        4,
      );
    }

    let wake_task_counts = cx.read_storage_array(&wake_task_counts);
    let empty_task_counts = cx.read_storage_array(&empty_task_counts);

    cx.submit_recorded_work_and_continue();

    let wake_counts = wake_task_counts.await.unwrap();
    let empty_counts = empty_task_counts.await.unwrap();

    let sleep_or_finished_counts = empty_counts
      .iter()
      .zip(wake_counts.iter())
      .zip(self.task_groups.iter())
      .map(|((empty, &wake), info)| {
        let full_size = info.max_in_flight * self.current_prepared_execution_size;

        (full_size - *empty as usize - wake as usize) as u32
      })
      .collect();

    TaskGraphExecutionStates {
      wake_counts,
      empty_counts,
      sleep_or_finished_counts,
    }
  }

  pub async fn debug_execution(
    &mut self,
    cx: &mut DeviceParallelComputeCtx<'_>,
  ) -> TaskGraphExecutionDebugInfo {
    self.task_groups.iter_mut().for_each(|task| {
      task.prepare_execution_and_compact_living_task(cx);
    });
    cx.flush_pass();

    let mut info = Vec::new();

    for task in &self.task_groups {
      let active_idx = task.resource.active_task_idx.debug_execution(cx).await;
      let empty_idx = task.resource.empty_index_pool.debug_execution(cx).await;

      let task_states = cx.read_buffer_bytes(&task.resource.task_pool.tasks.get_gpu_buffer_view());

      cx.submit_recorded_work_and_continue();
      let task_states = task_states.await.unwrap();

      info.push(TaskExecutionDebugInfo {
        active_idx,
        empty_idx,
        task_states,
      })
    }

    TaskGraphExecutionDebugInfo { info }
  }

  // todo, dispatch should in reverse order
  pub fn execute(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    dispatch_round_count: usize,
    source: &DeviceTaskGraphBuildSource,
  ) {
    let self_task_groups: &[TaskGroupExecutor] = &self.task_groups;
    // todo, this is unsound
    let self_task_groups: &'static [TaskGroupExecutor] =
      unsafe { std::mem::transmute(self_task_groups) };

    for _ in 0..dispatch_round_count {
      for (idx, task) in self.task_groups.iter_mut().enumerate() {
        let source = &source.tasks[idx];
        task.execute(cx, self_task_groups, source);
      }
    }
  }
}
