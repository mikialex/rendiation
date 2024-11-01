use crate::*;

#[derive(Clone)]
pub struct GPUWaveFrontComputeRaytracingBakedPipeline {
  pub(crate) inner: Arc<RwLock<GPUWaveFrontComputeRaytracingBakedPipelineInner>>,
}

pub struct GPUWaveFrontComputeRaytracingBakedPipelineInner {
  pub(crate) desc: GPURaytracingPipelineDescriptor,
  pub(crate) executor: Option<(u64, GPUWaveFrontComputeRaytracingExecutor)>,
}

pub struct GPUWaveFrontComputeRaytracingExecutor {
  pub(crate) graph: DeviceTaskGraphExecutor,
  pub(crate) tracer_read_back_bumper: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
  pub(crate) target_sbt_buffer: StorageBufferReadOnlyDataView<u32>,
}

impl GPUWaveFrontComputeRaytracingBakedPipelineInner {
  pub fn get_or_compile_executor(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
    sbt_sys: ShaderBindingTableDeviceInfo,
    init_size: usize,
  ) -> &mut GPUWaveFrontComputeRaytracingExecutor {
    let current_hash = self.desc.compute_hash();
    if let Some((hash, _)) = &mut self.executor {
      if current_hash != *hash {
        self.executor = None;
      }
    }
    let (_, exe) = self.executor.get_or_insert_with(|| {
      let exe = Self::compile_executor(&self.desc, cx, tlas_sys, sbt_sys, init_size);
      (current_hash, exe)
    });
    exe
  }

  fn compile_executor(
    desc: &GPURaytracingPipelineDescriptor,
    cx: &mut DeviceParallelComputeCtx,
    tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
    sbt_sys: ShaderBindingTableDeviceInfo,
    init_size: usize,
  ) -> GPUWaveFrontComputeRaytracingExecutor {
    let mut graph = DeviceTaskGraphBuildSource::default();

    let mut payload_max_u32_count = 0;

    // todo assert at least one for each stage will be defined
    let ray_gen_task_range_start = 1;
    let ray_gen_task_range_end = ray_gen_task_range_start + desc.ray_gen_shaders.len();

    let closest_task_range_start = ray_gen_task_range_end;
    let closest_task_range_end = closest_task_range_start + desc.closest_hit_shaders.len();
    let closest_tasks = desc
      .closest_hit_shaders
      .iter()
      .enumerate()
      .map(|(i, (_, ty))| {
        payload_max_u32_count = payload_max_u32_count.max(ty.u32_size_count());
        ((i + closest_task_range_start) as u32, ty.clone())
      })
      .collect();

    let missing_task_start = closest_task_range_end;
    let missing_task_end = missing_task_start + desc.miss_hit_shaders.len();
    let missing_tasks = desc
      .miss_hit_shaders
      .iter()
      .enumerate()
      .map(|(i, (_, ty))| {
        payload_max_u32_count = payload_max_u32_count.max(ty.u32_size_count());
        ((i + missing_task_start) as u32, ty.clone())
      })
      .collect();

    let info = TraceTaskMetaInfo {
      closest_tasks,
      missing_tasks,
      intersection_shaders: desc.intersection_shaders.clone(),
      any_hit_shaders: desc.any_hit_shaders.clone(),
      payload_max_u32_count,
    };

    let device = &cx.gpu.device;
    let target_sbt_buffer = StorageBufferReadOnlyDataView::create(device, &0);

    let payload_u32_len = init_size * 2 * (payload_max_u32_count as usize);
    let payload_bumper = Arc::new(RwLock::new(DeviceBumpAllocationInstance::new(
      payload_u32_len,
      device,
    )));

    let payload_read_back_bumper = Arc::new(RwLock::new(DeviceBumpAllocationInstance::new(
      payload_u32_len,
      device,
    )));

    let tracer_task = TraceTaskImpl {
      tlas_sys,
      sbt_sys,
      payload_bumper: payload_bumper.clone(),
      payload_read_back_bumper: payload_read_back_bumper.clone(),
      ray_info_bumper: DeviceBumpAllocationInstance::new(init_size * 2, device),
      info: Arc::new(info),
      current_sbt: target_sbt_buffer.clone(),
    };

    let mut ctx = AnyMap::default();
    ctx.register(TracingTaskSpawnerImplSource {
      payload_spawn_bumper: tracer_task.payload_bumper.clone(),
      payload_read_back: tracer_task.payload_read_back_bumper.clone(),
    });

    // create core tracer task as almost every other task depend on this one
    let trace_task_id = graph.define_task_dyn(
      Box::new(OpaqueTaskWrapper(tracer_task)) as OpaqueTask,
      TraceTaskSelfPayload::sized_ty(),
    );
    assert_eq!(trace_task_id, 0);

    for (stage, ty) in &desc.ray_gen_shaders {
      let task_id = graph.define_task_dyn(
        Box::new(OpaqueTaskWrapper(stage.build_device_future(&mut ctx))) as OpaqueTask,
        ty.clone(),
      );
      assert!((ray_gen_task_range_start..ray_gen_task_range_end).contains(&(task_id as usize)));
    }

    for (stage, ty) in &desc.closest_hit_shaders {
      let task_payload_ty = create_composite_task_payload_desc(
        graph.next_task_idx(),
        ty,
        &RayClosestHitCtxPayload::sized_ty(),
      );
      let task_id = graph.define_task_dyn(
        Box::new(OpaqueTaskWrapper(stage.build_device_future(&mut ctx))) as OpaqueTask,
        task_payload_ty,
      );
      assert!((closest_task_range_start..closest_task_range_end).contains(&(task_id as usize)));
    }

    for (stage, ty) in &desc.miss_hit_shaders {
      let task_payload_ty = create_composite_task_payload_desc(
        graph.next_task_idx(),
        ty,
        &RayMissHitCtxPayload::sized_ty(),
      );
      let task_id = graph.define_task_dyn(
        Box::new(OpaqueTaskWrapper(stage.build_device_future(&mut ctx))) as OpaqueTask,
        task_payload_ty,
      );
      assert!((missing_task_start..missing_task_end).contains(&(task_id as usize)));
    }

    let executor = graph.build(1, 1, cx);

    GPUWaveFrontComputeRaytracingExecutor {
      graph: executor,
      target_sbt_buffer,
      tracer_read_back_bumper: payload_read_back_bumper,
    }
  }
}

impl GPURaytracingPipelineProvider for GPUWaveFrontComputeRaytracingBakedPipeline {
  fn access_impl(&self) -> &dyn Any {
    self
  }
}
