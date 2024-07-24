use crate::*;

pub struct WavefrontExecutor {
  stages: Vec<WavefrontStageExecutor>,
  max_recursion_depth: u32,
  current_prepared_execution_size: (u32, u32, u32),
}

pub struct WavefrontExecutorBuildCtx;

impl DeviceStateProvider for WavefrontExecutorBuildCtx {
  fn provide_state<T>(&mut self) -> BoxedShaderLoadStore<T> {
    todo!()
  }
}

impl WavefrontExecutor {
  fn empty() -> Self {
    Self {
      stages: Default::default(),
      max_recursion_depth: 6,
      current_prepared_execution_size: (1, 1, 1),
    }
  }

  fn define_state<F>(
    &self,
    future: F,
    cx_provider: impl FnOnce(&mut WavefrontExecutorBuildCtx) -> F::Ctx,
  ) -> u32
  where
    F: ShaderFuture,
  {
    todo!()
  }
}

impl WavefrontExecutor {
  pub fn compile_from(desc: &GPURaytracingPipelineBuilder) -> Self {
    let mut executor = Self::empty();
    executor
  }
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
    todo!()
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

impl WavefrontExecutor {
  pub fn execute(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
    self.make_sure_execution_size_is_enough(gpu, dispatch_size);

    let mut encoder = gpu.create_encoder();

    encoder.compute_pass_scoped(|pass| {
      for _ in 0..self.max_recursion_depth {
        for stage in &self.stages {
          // pass.dispatch_workgroups_indirect(indirect_buffer, indirect_offset)
        }
      }
    });
    // todo check state states to make sure no task remains
  }
}

struct WavefrontStageExecutor {
  index: usize,
  depend_on: Vec<usize>,
  depend_by: Vec<usize>,
  tasks: GPUBufferView,
  batch_info: GPUBufferView,
  pipeline: GPUComputePipeline,
}
