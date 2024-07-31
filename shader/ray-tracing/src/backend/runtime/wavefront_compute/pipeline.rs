use crate::*;

impl GPURaytracingPipelineBuilder {
  pub fn compile_task_executor(&self) -> DeviceTaskGraphExecutor {
    let mut executor = DeviceTaskGraphExecutor::empty();
    executor
  }
}
