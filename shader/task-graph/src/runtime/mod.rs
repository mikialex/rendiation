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
  pub max_recursion_depth: usize,
  pub capacity: usize,
}

impl DeviceTaskGraphBuildSource {
  // todo, impl more conservative upper execute bound
  pub fn compute_conservative_dispatch_round_count(&self) -> usize {
    let max_required_poll_count = self
      .tasks
      .iter()
      .map(|v| v.task.required_poll_count())
      .max()
      .unwrap_or(1);
    self.max_recursion_depth * max_required_poll_count + 1
  }

  pub fn define_task<P, F>(&mut self, future: F) -> u32
  where
    F: ShaderFuture<Output = ()> + 'static,
    P: ShaderSizedValueNodeType,
  {
    self.define_task_dyn(
      Box::new(OpaqueTaskWrapper(future)) as OpaqueTask,
      P::sized_ty(),
    )
  }
  pub fn next_task_idx(&self) -> u32 {
    self.tasks.len() as u32
  }

  #[inline(never)]
  pub fn define_task_dyn(&mut self, task: OpaqueTask, payload_ty: ShaderSizedValueType) -> u32 {
    let task_type = self.tasks.len();

    self.tasks.push(TaskGroupBuildSource {
      payload_ty,
      self_task_idx: task_type,
      task,
    });

    task_type as u32
  }

  pub fn build(&self, cx: &mut DeviceParallelComputeCtx) -> DeviceTaskGraphExecutor {
    let init_size = self.max_recursion_depth * self.capacity;
    let mut task_group_shared_info = Vec::new();
    for _ in 0..self.tasks.len() {
      task_group_shared_info.push((Default::default(), Default::default()));
    }

    let mut pre_builds = Vec::new();
    let mut task_group_sources = Vec::new();

    for (i, task_build_source) in self.tasks.iter().enumerate() {
      let pre_build =
        TaskGroupExecutor::pre_build(task_build_source, i, &mut task_group_shared_info);

      let resource = TaskGroupExecutorResource::create_with_size(
        init_size,
        pre_build.state_to_resolve.meta_info(),
        task_build_source.payload_ty.clone(),
        cx,
      );

      task_group_sources.push(resource);
      pre_builds.push(pre_build);
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
      max_recursion_depth: self.max_recursion_depth,
      current_prepared_execution_size: self.capacity,
    }
  }
}

pub struct DeviceTaskGraphExecutor {
  task_groups: Vec<TaskGroupExecutor>,
  max_recursion_depth: usize,
  current_prepared_execution_size: usize,
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
  /// T must match given task_id's payload type
  ///
  /// From perspective of performance, this method can be implemented as a special task
  /// polling, but for consistency and simplicity, we implemented as a standalone task allocation procedure.
  pub fn dispatch_allocate_init_task<T: ShaderSizedValueNodeType>(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    task_count: u32,
    task_id: u32,
    task_spawner: impl FnOnce(Node<u32>) -> Node<T> + 'static,
  ) {
    let device = &cx.gpu.device;
    let task_group = &self.task_groups[task_id as usize];

    let dispatch_size_buffer = create_gpu_readonly_storage(&task_count, device);

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
          .spawn_new_task_dyn(
            payload.cast_untyped_node(),
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

  pub async fn read_back_execution_states<'a>(
    &mut self,
    cx: &mut DeviceParallelComputeCtx<'a>,
  ) -> TaskGraphExecutionStates {
    self.task_groups.iter_mut().for_each(|task| {
      task.prepare_execution(cx);
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
      cx.encoder.copy_buffer_to_buffer(
        task.resource.active_task_idx.current_size.buffer.gpu(),
        0,
        wake_task_counts.buffer.gpu(),
        i as u64 * 4,
        4,
      );
      cx.encoder.copy_buffer_to_buffer(
        task.resource.empty_index_pool.current_size.buffer.gpu(),
        0,
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

    // minus one is for the default task
    let full_size = self.max_recursion_depth * self.current_prepared_execution_size * 2 - 1;

    let sleep_or_finished_counts = empty_counts
      .iter()
      .zip(wake_counts.iter())
      .map(|(empty, &wake)| (full_size - *empty as usize - wake as usize) as u32)
      .collect();

    TaskGraphExecutionStates {
      wake_counts,
      empty_counts,
      sleep_or_finished_counts,
    }
  }

  pub async fn debug_execution<'a>(
    &mut self,
    cx: &mut DeviceParallelComputeCtx<'a>,
  ) -> TaskGraphExecutionDebugInfo {
    self.task_groups.iter_mut().for_each(|task| {
      task.prepare_execution(cx);
    });
    cx.flush_pass();

    let mut info = Vec::new();

    for task in &self.task_groups {
      let active_idx = task.resource.active_task_idx.debug_execution(cx).await;
      let empty_idx = task.resource.empty_index_pool.debug_execution(cx).await;

      let task_states = cx.read_buffer_bytes(&task.resource.task_pool.tasks);

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
