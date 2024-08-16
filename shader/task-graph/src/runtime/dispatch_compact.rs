use super::*;

#[derive(Clone)]
pub struct ActiveTaskCompact {
  pub alive_size: StorageBufferDataView<u32>,
  pub active_tasks: StorageBufferReadOnlyDataView<[u32]>,
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
    let active_tasks = builder.bind_by(&self.active_tasks);
    let size = builder.bind_by(&self.alive_size);
    let task_pool = self.task_pool.build_shader(builder);
    let inner = (active_tasks, size, task_pool);

    RealAdhocInvocationResult {
      inner,
      compute: Box::new(|inner, id| {
        let (r, is_valid) = inner.0.invocation_logic(id);

        //  check task_pool access is valid?
        let r = inner.2.poll_task_is_finished(r);

        (r, is_valid)
      }),
      size: Box::new(|inner| (inner.1.load(), val(0), val(0)).into()),
    }
    .into_boxed()
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
