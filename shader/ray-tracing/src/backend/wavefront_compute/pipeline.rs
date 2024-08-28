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

    for (stage, ty) in &desc.ray_gen_shaders {
      executor.define_task_dyn(
        Box::new(OpaqueTaskWrapper(stage.build_device_future())) as OpaqueTask,
        ty.clone(),
        device,
        init_pass,
      );
    }

    for (stage, ty) in &desc.closest_hit_shaders {
      executor.define_task_dyn(
        Box::new(OpaqueTaskWrapper(stage.build_device_future())) as OpaqueTask,
        ty.clone(),
        device,
        init_pass,
      );
    }

    for (stage, ty) in &desc.miss_hit_shaders {
      executor.define_task_dyn(
        Box::new(OpaqueTaskWrapper(stage.build_device_future())) as OpaqueTask,
        ty.clone(),
        device,
        init_pass,
      );
    }

    TraceTaskMetaInfo {
      closest_tasks: todo!(),
      missing_tasks: todo!(),
      intersection_shaders: desc.intersection_shaders.clone(),
      any_hit_shaders: desc.any_hit_shaders.clone(),
      payload_max_u32_count: todo!(),
    };

    todo!();
  }
}

impl GPURaytracingPipelineProvider for GPUWaveFrontComputeRaytracingBakedPipeline {
  fn access_impl(&mut self) -> &mut dyn Any {
    self
  }
}
