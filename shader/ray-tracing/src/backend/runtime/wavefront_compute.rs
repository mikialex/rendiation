use rendiation_webgpu::{GPUBufferView, GPUCommandEncoder, GPUComputePipeline};

pub struct WavefrontExecutor {
  stages: Vec<WavefrontStageExecutor>,
}

impl WavefrontExecutor {
  pub fn execute(&self, encoder: &mut GPUCommandEncoder, max_recursion_depth: u32) {
    encoder.compute_pass_scoped(|pass| {
      for _ in 0..max_recursion_depth {
        for stage in &self.stages {
          // pass.dispatch_workgroups_indirect(indirect_buffer, indirect_offset)
        }
      }
    });
    // todo check state states to make sure no task remains
  }
}

pub struct WavefrontStageExecutor {
  tasks: GPUBufferView,
  batch_info: GPUBufferView,
  pipeline: GPUComputePipeline,
}
