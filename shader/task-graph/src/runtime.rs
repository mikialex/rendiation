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
  task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  current_task_idx: Node<u32>,
  self_task_type: usize,
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
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self
      .state_builder
      .create_or_reconstruct_inline_state(default)
  }

  fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> StorageNode<T> {
    let current = self.current_task_idx;
    let task_group = self.get_or_create_task_group_instance(self.self_task_type);
    task_group.task_pool.read_payload(current)
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
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self
      .inner
      .write()
      .create_or_reconstruct_inline_state(default)
  }

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
    let task_type = self.task_groups.len();

    let mut task_group_sources: Vec<_> = self.task_groups.iter().map(|x| &x.resource).collect();

    let resource = TaskGroupExecutorResource {
      alive_task_idx: todo!(),
      main_dispatch_size: todo!(),
      new_removed_task_idx: todo!(),
      empty_index_pool: todo!(),
      task_pool: todo!(),
    };

    task_group_sources.push(&resource);

    let ctx = DeviceTaskSystemBuildCtxImpl {
      compute_cx: &mut compute_cx,
      state_builder: Default::default(),
      task_group_sources,
      tasks: Default::default(),
      current_task_idx: todo!(),
      self_task_type: task_type,
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

    let task_executor = TaskGroupExecutor {
      index: task_type,
      task_poll_pipeline,
      resource,
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
      s.resource
        .resize(gpu, dispatch_size, self.max_recursion_depth)
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
  resource: TaskGroupExecutorResource,
}

struct TaskGroupExecutorResource {
  alive_task_idx: DeviceBumpAllocationInstance<u32>,
  main_dispatch_size: StorageBufferDataView<DispatchIndirectArgs>,

  new_removed_task_idx: DeviceBumpAllocationInstance<u32>,
  empty_index_pool: DeviceBumpAllocationInstance<u32>,
  task_pool: GPUBufferResourceView, // (task_state, payload)
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
  pub fn resize(&mut self, gpu: &GPU, size: (u32, u32, u32), max_recursion_depth: u32) {
    todo!()
  }
  pub fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> TaskGroupDeviceInvocationInstance {
    TaskGroupDeviceInvocationInstance {
      new_removed_task_idx: self.new_removed_task_idx.build_allocator_shader(compute_cx),
      empty_index_pool: self
        .new_removed_task_idx
        .build_deallocator_shader(compute_cx),
      task_pool: compute_cx.entry_by(|cx| {
        cx.bindgroups().binding_dyn(ShaderBindingDescriptor {
          should_as_storage_buffer_if_is_buffer_like: true,
          ty: ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(
            todo!(),
          ))),
          writeable_if_storage: true,
        });
        todo!()
      }),
    }
  }
}

// struct Task {
//   is_finished_ bool,
//   payload: P,
//   state: S,
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
      // error report, unreachable?
    });
    idx
  }

  pub fn cleanup_finished_task_state_and_payload(&self, task: Node<u32>) {
    let (_, success) = self.new_removed_task_idx.bump_allocate(task);
    if_by(success.not(), || {
      // error report, unreachable?
    });
  }

  pub fn poll_task_is_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self.task_pool.poll_task_is_finished(task_id)
  }
}

struct TaskPoolInvocationInstance {
  pool: StorageNode<[AnyType]>,
  state_desc: DynamicTypeBaked,
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
    for (i, (_, v)) in self.state_desc.fields.iter().enumerate() {
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
