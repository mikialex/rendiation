use crate::*;

impl GPURaytracingPipelineBuilder {
  pub fn compile_task_executor(
    &self,
    device: &GPUDevice,
    init_size: usize,
  ) -> DeviceTaskGraphExecutor {
    let mut executor = DeviceTaskGraphExecutor::empty();

    executor.define_task(BaseDeviceFuture::default(), || (), device, init_size);

    executor
  }
}
