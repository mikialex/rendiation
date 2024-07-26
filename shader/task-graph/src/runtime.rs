use crate::*;

pub struct DeviceTaskGraphExecutor {
  task_groups: Vec<TaskGroupExecutor>,
  max_recursion_depth: u32,
  current_prepared_execution_size: (u32, u32, u32),
}

pub struct DeviceTaskSystemBuildCtx<'a> {
  inner: RwLock<DeviceTaskSystemBuildCtxImpl<'a>>,
}

pub struct DeviceTaskSystemBuildCtxImpl<'a> {
  compute_cx: &'a mut ShaderComputePipelineBuilder,
  state_builder: DynamicTypeBuilder,
  task_group_sources: &'a Vec<TaskGroupExecutor>,
  depend_by: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
}

impl<'a> DeviceTaskSystemBuildCtxImpl<'a> {
  fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> &mut TaskGroupDeviceInvocationInstance {
    self.depend_by.entry(task_type).or_insert_with(|| {
      let source = &self.task_group_sources[task_type];
      source.build_shader(self.compute_cx)
    })
  }
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self
      .state_builder
      .create_or_reconstruct_inline_state(default)
  }

  fn spawn_task<T>(&mut self, task_type: usize, argument: Node<T>) -> Node<u32> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    task_group.spawn_new_task(argument)
  }

  fn poll_task<T>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    let finished = task_group.poll_task_is_finished(task_id);
    if_by(finished, || {
      argument_read_back(task_group.read_back_payload(task_id));
      task_group.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }
}

impl<'a> DeviceTaskSystemContextProvider for DeviceTaskSystemBuildCtx<'a> {
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self
      .inner
      .write()
      .create_or_reconstruct_inline_state(default)
  }

  fn spawn_task<T>(&self, task_type: usize, argument: Node<T>) -> Node<u32> {
    self.inner.write().spawn_task(task_type, argument)
  }

  fn poll_task<T>(
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
      current_prepared_execution_size: (1, 1, 1),
    }
  }

  pub fn define_task<F>(
    &mut self,
    future: F,
    cx_provider: impl FnOnce(&mut DeviceTaskSystemBuildCtx) -> F::Ctx,
    device: &GPUDevice,
  ) -> u32
  where
    F: DeviceFuture<Output = ()>,
  {
    let mut compute_cx = compute_shader_builder();
    let ctx = DeviceTaskSystemBuildCtxImpl {
      compute_cx: &mut compute_cx,
      state_builder: Default::default(),
      task_group_sources: &self.task_groups,
      depend_by: Default::default(),
    };
    let mut ctx = DeviceTaskSystemBuildCtx {
      inner: RwLock::new(ctx),
    };
    let mut ctx = cx_provider(&mut ctx);

    let state = future.create_or_reconstruct_state(&mut ctx);

    let task_poll_pipeline = compute_cx
      .entry(|cx| {
        let poll_result = future.poll(&state, &ctx);
        if_by(poll_result.is_ready, || {
          //
        });
      })
      .create_compute_pipeline(device)
      .unwrap();

    let task_type = self.task_groups.len();
    let task_executor = TaskGroupExecutor {
      index: task_type,
      alive_task_idx: todo!(),
      new_removed_task_idx: todo!(),
      empty_index_pool: todo!(),
      task_pool: todo!(),
      device_size: todo!(),
      task_poll_pipeline,
      main_dispatch_size: todo!(),
    };
    self.task_groups.push(task_executor);

    task_type as u32
  }
}

impl DeviceTaskGraphExecutor {
  pub fn set_execution_size(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
    let dispatch_size = (
      dispatch_size.0.min(1),
      dispatch_size.1.min(1),
      dispatch_size.2.min(1),
    );
    if self.current_prepared_execution_size == dispatch_size {
      return;
    }
    self.current_prepared_execution_size = dispatch_size;
    for s in &mut self.task_groups {
      s.resize(gpu, dispatch_size, self.max_recursion_depth)
    }
  }

  fn make_sure_execution_size_is_enough(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
    let is_contained = self.current_prepared_execution_size.0 <= dispatch_size.0
      && self.current_prepared_execution_size.1 <= dispatch_size.1
      && self.current_prepared_execution_size.2 <= dispatch_size.2;

    if !is_contained {
      self.set_execution_size(gpu, dispatch_size)
    }
  }
}

impl DeviceTaskGraphExecutor {
  pub fn execute(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
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
  index: usize,
  task_poll_pipeline: GPUComputePipeline,

  alive_task_idx: DeviceBumpAllocationInstance<u32>,
  main_dispatch_size: StorageBufferDataView<DispatchIndirectArgs>,

  new_removed_task_idx: DeviceBumpAllocationInstance<u32>,
  empty_index_pool: DeviceBumpAllocationInstance<u32>,
  task_pool: DeviceUntypedBumpAllocationInstance, // (task_state, payload)
  device_size: GPUBufferView,
}

impl TaskGroupExecutor {
  pub fn resize(&mut self, gpu: &GPU, size: (u32, u32, u32), max_recursion_depth: u32) {
    todo!()
  }

  pub fn execute(&self, pass: &mut GPUComputePass) {
    pass.set_pipeline_owned(&self.task_poll_pipeline);
    // pass.set_bind_group(index, bind_group, offsets)
    // todo, prepare size args
    // pass.dispatch_workgroups_indirect(&self.main_dispatch_size.buffer.gpu(), 0);
  }

  pub fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> TaskGroupDeviceInvocationInstance {
    todo!()
  }
}

// struct Task {
//   payload: P,
//   state: S,
//   parent_task: Option<(Node<u32>, Node<u32>)>,
//   child_wake_counter: Node<DeviceAtomic<u32>>
// }

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug)]
pub struct DispatchIndirectArgs {
  pub x: u32,
  pub y: u32,
  pub z: u32,
}

pub struct TaskGroupDeviceInvocationInstance {
  index: usize,
  /// point to task pool
  alive_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  new_removed_task_idx: DeviceBumpAllocationInvocationInstance<u32>,
  empty_index_pool: DeviceBumpAllocationInvocationInstance<u32>,
  task_pool: DeviceUntypedBumpAllocationInvocationInstance,
  state_desc: DynamicTypeBaked,
}

impl TaskGroupDeviceInvocationInstance {
  pub fn spawn_new_task<T>(&self, payload: Node<T>) -> Node<u32> {
    todo!()
  }

  pub fn read_back_payload<T>(&self, task: Node<u32>) -> Node<T> {
    todo!()
  }

  pub fn cleanup_finished_task_state_and_payload(&self, task: Node<u32>) {
    todo!()
  }

  pub fn poll_task_is_finished(&self, task_id: Node<u32>) -> Node<bool> {
    todo!()
  }
}
