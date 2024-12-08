use pollster::block_on;

use crate::*;

#[derive(Clone)]
pub struct GPUWaveFrontComputeRaytracingBakedPipeline {
  pub(crate) inner: Arc<RwLock<GPUWaveFrontComputeRaytracingBakedPipelineInner>>,
}

impl GPURaytracingPipelineExecutorImpl for GPUWaveFrontComputeRaytracingBakedPipeline {
  fn access_impl(&self) -> &dyn Any {
    self
  }
  fn assert_is_empty(&self, gpu: &GPU) {
    let mut inner = self.inner.write();
    if let Some((_, _, executor)) = &mut inner.executor {
      let mut encoder = gpu.create_encoder();
      let mut cx = DeviceParallelComputeCtx::new(gpu, &mut encoder);
      let states = block_on(executor.graph.read_back_execution_states(&mut cx));
      if !states.is_empty() {
        panic!("pipeline is not empty:\n {:?}", states);
      }
    }
  }
}

pub struct GPUWaveFrontComputeRaytracingBakedPipelineInner {
  pub(crate) executor: Option<(u64, usize, GPUWaveFrontComputeRaytracingExecutor)>,
}

pub struct GPUWaveFrontComputeRaytracingExecutor {
  pub(crate) graph: DeviceTaskGraphExecutor,
  pub(crate) ray_gen_task_idx: u32,
  pub(crate) target_sbt_buffer: StorageBufferReadOnlyDataView<u32>,
  pub(crate) launch_size_buffer: StorageBufferReadOnlyDataView<Vec3<u32>>,
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Copy, Clone, ShaderStruct)]
pub struct SbtTaskMapping {
  pub ray_gen_start: u32, // handle k -> task id (k + ray_gen_start)
  pub closest_start: u32, // handle k -> task id (k + closest_start)
  pub miss_start: u32,    // handle k -> task id (k + miss_start)
}
impl SbtTaskMapping {
  pub fn new(ray_gen_start: u32, closest_start: u32, miss_start: u32) -> Self {
    println!(
      "ray_gen_start: {}, closest_start: {}, miss_start: {}",
      ray_gen_start, closest_start, miss_start
    );
    Self {
      ray_gen_start,
      closest_start,
      miss_start,
      ..Zeroable::zeroed()
    }
  }
}
impl SbtTaskMappingShaderAPIInstance {
  pub fn get_ray_gen_task(&self, ray_gen_shader_index: Node<u32>) -> Node<u32> {
    ray_gen_shader_index + self.ray_gen_start
  }
  pub fn get_closest_task(&self, closest_shader_index: Node<u32>) -> Node<u32> {
    closest_shader_index + self.closest_start
  }
  pub fn get_miss_task(&self, miss_shader_index: Node<u32>) -> Node<u32> {
    miss_shader_index + self.miss_start
  }
}

impl GPUWaveFrontComputeRaytracingBakedPipelineInner {
  pub fn get_or_compile_executor(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    desc: &GPURaytracingPipelineAndBindingSource,
    tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
    sbt_sys: ShaderBindingTableDeviceInfo,
    required_size: usize,
  ) -> &mut GPUWaveFrontComputeRaytracingExecutor {
    let current_hash = desc.compute_hash();
    if let Some((hash, size, _)) = &mut self.executor {
      if current_hash != *hash || *size < required_size {
        self.executor = None;
      }
    }
    let (_, _, exe) = self.executor.get_or_insert_with(|| {
      let exe = Self::compile_executor(desc, cx, tlas_sys, sbt_sys, required_size);
      (current_hash, required_size, exe)
    });
    exe
  }

  fn compile_executor(
    desc: &GPURaytracingPipelineAndBindingSource,
    cx: &mut DeviceParallelComputeCtx,
    tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
    sbt_sys: ShaderBindingTableDeviceInfo,
    init_size: usize,
  ) -> GPUWaveFrontComputeRaytracingExecutor {
    let mut graph = DeviceTaskGraphBuildSource::default();

    let mut payload_max_u32_count = 0;

    // todo assert at least one for each stage will be defined
    let ray_gen_task_range_start = 1;
    let ray_gen_task_range_end = ray_gen_task_range_start + desc.ray_gen.len();

    let closest_task_range_start = ray_gen_task_range_end;
    let closest_task_range_end = closest_task_range_start + desc.closest_hit.len();
    let closest_tasks = desc
      .closest_hit
      .iter()
      .enumerate()
      .map(|(i, s)| {
        let ty = &s.user_defined_payload_input_ty;
        payload_max_u32_count = payload_max_u32_count.max(ty.u32_size_count());
        ((i + closest_task_range_start) as u32, ty.clone())
      })
      .collect();

    let missing_task_start = closest_task_range_end;
    let missing_task_end = missing_task_start + desc.miss_hit.len();
    let missing_tasks = desc
      .miss_hit
      .iter()
      .enumerate()
      .map(|(i, s)| {
        let ty = &s.user_defined_payload_input_ty;
        payload_max_u32_count = payload_max_u32_count.max(ty.u32_size_count());
        ((i + missing_task_start) as u32, ty.clone())
      })
      .collect();

    let info = TraceTaskMetaInfo {
      closest_tasks,
      missing_tasks,
      intersection_shaders: desc.intersection.clone(),
      any_hit_shaders: desc.any_hit.clone(),
      payload_max_u32_count,
    };

    let device = &cx.gpu.device;
    let target_sbt_buffer = StorageBufferReadOnlyDataView::create(device, &0);
    let sbt_task_mapping_buffer = StorageBufferReadOnlyDataView::create(
      device,
      &SbtTaskMapping::new(
        ray_gen_task_range_start as u32,
        closest_task_range_start as u32,
        missing_task_start as u32,
      ),
    );
    // written in trace_ray. see RayLaunchSizeBuffer
    let launch_size_buffer = StorageBufferReadOnlyDataView::create(device, &vec3(0, 0, 0));

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
      sbt_task_mapping: sbt_task_mapping_buffer,
    };

    let mut ctx = AnyMap::default();
    ctx.register(TracingTaskSpawnerImplSource {
      payload_spawn_bumper: tracer_task.payload_bumper.clone(),
      payload_read_back: tracer_task.payload_read_back_bumper.clone(),
    });
    ctx.register(RayLaunchSizeBuffer {
      launch_size: launch_size_buffer.clone(),
    });

    // create core tracer task as almost every other task depend on this one
    let trace_task_id = graph.define_task_dyn(
      Box::new(OpaqueTaskWrapper(tracer_task)) as OpaqueTask,
      TraceTaskSelfPayload::sized_ty(),
    );
    assert_eq!(trace_task_id, 0);

    assert_eq!(desc.ray_gen.len(), 1);
    let mut ray_gen_task_idx = 0;
    for s in &desc.ray_gen {
      let task_id = graph.define_task_dyn(
        Box::new(OpaqueTaskWrapper(s.logic.build_device_future(&mut ctx))) as OpaqueTask,
        s.user_defined_payload_input_ty.clone(),
      );
      ray_gen_task_idx = task_id;
      assert!((ray_gen_task_range_start..ray_gen_task_range_end).contains(&(task_id as usize)));
    }

    for s in &desc.closest_hit {
      let task_payload_ty = create_composite_task_payload_desc(
        graph.next_task_idx(),
        &s.user_defined_payload_input_ty,
        &RayClosestHitCtxPayload::sized_ty(),
      );
      let task_id = graph.define_task_dyn(
        Box::new(OpaqueTaskWrapper(s.logic.build_device_future(&mut ctx))) as OpaqueTask,
        task_payload_ty,
      );
      assert!((closest_task_range_start..closest_task_range_end).contains(&(task_id as usize)));
    }

    for s in &desc.miss_hit {
      let task_payload_ty = create_composite_task_payload_desc(
        graph.next_task_idx(),
        &s.user_defined_payload_input_ty,
        &RayMissHitCtxPayload::sized_ty(),
      );
      let task_id = graph.define_task_dyn(
        Box::new(OpaqueTaskWrapper(s.logic.build_device_future(&mut ctx))) as OpaqueTask,
        task_payload_ty,
      );
      assert!((missing_task_start..missing_task_end).contains(&(task_id as usize)));
    }

    let mut executor = graph.build(init_size, desc.max_recursion_depth as usize, cx);

    executor.set_task_before_execution_hook(TRACING_TASK_INDEX, move |cx, _| {
      payload_read_back_bumper.read().reset(cx);
    });

    executor.set_task_after_execution_hook(TRACING_TASK_INDEX, move |cx, _| {
      payload_bumper.read().reset(cx);
    });

    GPUWaveFrontComputeRaytracingExecutor {
      ray_gen_task_idx,
      graph: executor,
      target_sbt_buffer,
      launch_size_buffer,
    }
  }
}
