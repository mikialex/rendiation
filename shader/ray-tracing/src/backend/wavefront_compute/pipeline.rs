use crate::*;

pub struct GPUWaveFrontComputeRaytracingBakedPipeline {
  graph: DeviceTaskGraphExecutor,
}

impl GPUWaveFrontComputeRaytracingBakedPipeline {
  pub fn compile(
    desc: &GPURaytracingPipelineDescriptor,
    device: &GPUDevice,
    init_size: usize,
  ) -> Self {
    let mut executor = DeviceTaskGraphExecutor::new(1, 1);

    // executor.registry.

    let init_pass = todo!();

    // executor.define_task(
    //   BaseDeviceFuture::default(),
    //   || (),
    //   device,
    //   init_size,
    //   init_pass,
    // );

    // for ray_gen in &self.ray_gen_shaders {
    //   executor.define_task(ray_gen.build_device_future(), device, init_pass);
    // }

    // for closet in &self.closest_hit_shaders {
    //   executor.define_task::<_, _>(closet.build_device_future(), device, init_pass);
    // }

    todo!();
  }
}

impl GPURaytracingPipelineProvider for GPUWaveFrontComputeRaytracingBakedPipeline {
  fn access_impl(&mut self) -> &mut dyn Any {
    self
  }
}
