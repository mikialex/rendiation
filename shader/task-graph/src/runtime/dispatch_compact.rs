use super::*;

// todo, impl for T
#[derive(Clone)]
pub struct ParallelComputeFromAbstractStorageBuffer(pub BoxedAbstractStorageBuffer<[u32]>);

impl DeviceParallelCompute<Node<u32>> for ParallelComputeFromAbstractStorageBuffer {
  fn execute_and_expose(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn result_size(&self) -> u32 {
    let byte_size = self.0.get_gpu_buffer_view().view_byte_size();
    u64::from(byte_size) as u32 / 4
  }
}
impl DeviceParallelComputeIO<u32> for ParallelComputeFromAbstractStorageBuffer {}

/// this is a little dangerous
impl ShaderHashProvider for ParallelComputeFromAbstractStorageBuffer {
  shader_hash_type_id! {}
}
impl DeviceInvocationComponent<Node<u32>> for ParallelComputeFromAbstractStorageBuffer {
  fn work_size(&self) -> Option<u32> {
    Some(self.result_size())
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let ptr = builder.bind_abstract_storage(&self.0);
    Box::new(ptr)
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind_abstract_storage(&self.0);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
}

#[derive(Clone)]
pub struct ActiveTaskCompact {
  pub active_size: BoxedAbstractStorageBuffer<u32>,
  pub active_tasks: BoxedAbstractStorageBuffer<[u32]>,
  pub task_pool: TaskPool,
}

impl DeviceParallelCompute<Node<bool>> for ActiveTaskCompact {
  fn execute_and_expose(
    &self,
    _: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<bool>>> {
    Box::new(self.clone())
  }

  fn result_size(&self) -> u32 {
    let byte_size = self.active_tasks.get_gpu_buffer_view().view_byte_size();
    u64::from(byte_size) as u32 / 4
  }
}
impl DeviceParallelComputeIO<bool> for ActiveTaskCompact {}

impl ShaderHashProvider for ActiveTaskCompact {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.task_pool.hash_pipeline(hasher)
  }
  shader_hash_type_id! {}
}

impl DeviceInvocationComponent<Node<bool>> for ActiveTaskCompact {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<bool>>> {
    let active_tasks = builder.bind_abstract_storage(&self.active_tasks);
    let size = builder.bind_abstract_storage(&self.active_size);
    let task_pool = self.task_pool.build_shader(builder);
    let inner = (active_tasks, size, task_pool);

    RealAdhocInvocationResult {
      inner,
      compute: Box::new(|inner, id| {
        let (r, is_valid) = inner.0.invocation_logic(id);
        let is_valid = is_valid.and(id.x().less_than(inner.1.load()));

        let rr = val(false).make_local_var();

        if_by(is_valid, || {
          let task_state = inner.2.rw_task_state(r).load();

          if_by(task_state.equals(TASK_STATUE_FLAG_SLEEPING), || {
            inner
              .2
              .rw_task_state(r)
              .store(TASK_STATUE_FLAG_NOT_FINISHED_SLEEP);
          });

          let is_task_unfinished_waken = task_state.equals(TASK_STATUE_FLAG_NOT_FINISHED_WAKEN);

          rr.store(is_task_unfinished_waken)
        });
        (rr.load(), is_valid)
      }),
      size: Box::new(|inner| (inner.1.load(), val(0), val(0)).into()),
    }
    .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind_abstract_storage(&self.active_tasks);
    builder.bind_abstract_storage(&self.active_size);
    self.task_pool.bind(builder);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn work_size(&self) -> Option<u32> {
    None
  }
}
