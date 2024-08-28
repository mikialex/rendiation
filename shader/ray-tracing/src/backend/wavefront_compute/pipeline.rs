use crate::*;

pub struct GPUWaveFrontComputeRaytracingBakedPipeline {
  pub(crate) graph: DeviceTaskGraphExecutor,
  pub(crate) target_sbt_buffer: StorageBufferReadOnlyDataView<u32>,
}

impl GPUWaveFrontComputeRaytracingBakedPipeline {
  pub fn compile(
    tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
    sbt_sys: ShaderBindingTableDeviceInfo,
    desc: &GPURaytracingPipelineDescriptor,
    device: &GPUDevice,
    init_size: usize,
    init_pass: &mut GPUComputePass,
  ) -> Self {
    let mut executor = DeviceTaskGraphExecutor::new(1, 1);

    // todo
    // executor.registry.register();

    let mut payload_max_u32_count = 0;

    let closest_task_range_base = desc.ray_gen_shaders.len();
    let closest_tasks = desc
      .closest_hit_shaders
      .iter()
      .enumerate()
      .map(|(i, (_, ty))| {
        payload_max_u32_count = payload_max_u32_count.max(ty.u32_size_count());
        ((i + closest_task_range_base) as u32, ty.clone())
      })
      .collect();

    let missing_tasks_base = closest_task_range_base + desc.closest_hit_shaders.len();
    let missing_tasks = desc
      .closest_hit_shaders
      .iter()
      .enumerate()
      .map(|(i, (_, ty))| {
        payload_max_u32_count = payload_max_u32_count.max(ty.u32_size_count());
        ((i + missing_tasks_base) as u32, ty.clone())
      })
      .collect();

    let info = TraceTaskMetaInfo {
      closest_tasks,
      missing_tasks,
      intersection_shaders: desc.intersection_shaders.clone(),
      any_hit_shaders: desc.any_hit_shaders.clone(),
      payload_max_u32_count,
    };

    let target_sbt_buffer = StorageBufferReadOnlyDataView::create(device, &0);

    let payload_u32_len = init_size * 2 * (payload_max_u32_count as usize);
    let tracer_task = TraceTaskImpl {
      tlas_sys,
      sbt_sys,
      payload_bumper: DeviceBumpAllocationInstance::new(payload_u32_len, device),
      payload_read_back_bumper: DeviceBumpAllocationInstance::new(payload_u32_len, device),
      ray_info_bumper: DeviceBumpAllocationInstance::new(init_size * 2, device),
      info: Arc::new(info),
      current_sbt: target_sbt_buffer.clone(),
    };

    // create core tracer task as almost every other task depend on this one
    executor.define_task_dyn(
      Box::new(OpaqueTaskWrapper(tracer_task)) as OpaqueTask,
      TraceTaskSelfPayload::sized_ty(),
      device,
      init_pass,
    );

    for (stage, ty) in &desc.ray_gen_shaders {
      executor.define_task_dyn(
        // todo use given wrapper
        Box::new(OpaqueTaskWrapper(stage.build_device_future())) as OpaqueTask,
        ty.clone(),
        device,
        init_pass,
      );
    }

    for (stage, ty) in &desc.closest_hit_shaders {
      executor.define_task_dyn(
        // todo use given wrapper
        Box::new(OpaqueTaskWrapper(stage.build_device_future())) as OpaqueTask,
        ty.clone(),
        device,
        init_pass,
      );
    }

    for (stage, ty) in &desc.miss_hit_shaders {
      executor.define_task_dyn(
        // todo use given wrapper
        Box::new(OpaqueTaskWrapper(stage.build_device_future())) as OpaqueTask,
        ty.clone(),
        device,
        init_pass,
      );
    }

    GPUWaveFrontComputeRaytracingBakedPipeline {
      graph: executor,
      target_sbt_buffer,
    }
  }
}

impl GPURaytracingPipelineProvider for GPUWaveFrontComputeRaytracingBakedPipeline {
  fn access_impl(&mut self) -> &mut dyn Any {
    self
  }
}
