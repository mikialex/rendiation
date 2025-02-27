use pollster::block_on;

use crate::*;

#[derive(Clone, Default)]
pub struct GPUWaveFrontComputeRaytracingExecutorImpl {
  pub(crate) inner: Arc<RwLock<GPUWaveFrontComputeRaytracingExecutorInternal>>,
}

impl GPURaytracingPipelineExecutorImpl for GPUWaveFrontComputeRaytracingExecutorImpl {
  fn access_impl(&self) -> &dyn Any {
    self
  }
  fn assert_is_empty(&self, gpu: &GPU) {
    let mut inner = self.inner.write();
    if let Some((_, executor)) = &mut inner.executor {
      let mut encoder = gpu.create_encoder();
      let mut cx = DeviceParallelComputeCtx::new(gpu, &mut encoder);
      let states = block_on(executor.graph_executor.read_back_execution_states(&mut cx));
      if !states.is_empty() {
        panic!("pipeline is not empty:\n {:?}", states);
      }
    }
  }
}

#[derive(Default)]
pub struct GPUWaveFrontComputeRaytracingExecutorInternal {
  executor: Option<(u64, GPUWaveFrontComputeRaytracingExecutor)>,
  resource: Option<TraceTaskResource>,
}

pub struct GPUWaveFrontComputeRaytracingExecutor {
  pub(crate) graph_executor: DeviceTaskGraphExecutor,
  pub(crate) resource: TraceTaskResource,
}

impl GPUWaveFrontComputeRaytracingExecutorInternal {
  pub fn get_or_compile_task_executor_and_task_source(
    &mut self,
    cx: &mut DeviceParallelComputeCtx,
    source: &GPURaytracingPipelineAndBindingSource,
    tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
    sbt_sys: ShaderBindingTableDeviceInfo,
    size: u32,
  ) -> (
    &mut GPUWaveFrontComputeRaytracingExecutor,
    DeviceTaskGraphBuildSource,
  ) {
    let current_hash = source.compute_hash(size);
    // todo, optimization size only change by skip shader recompile.
    if let Some((hash, _)) = &mut self.executor {
      if current_hash != *hash {
        self.executor = None;
        self.resource = None;
      }
    }

    let (task_graph, trace_resource) =
      create_task_graph(source, tlas_sys, sbt_sys, &mut self.resource, &cx.gpu, size);

    let (_, exe) = self.executor.get_or_insert_with(|| {
      let payload_read_back_bumper = trace_resource.payload_read_back_bumper.clone();
      let payload_bumper = trace_resource.payload_bumper.clone();

      let mut exe = task_graph.build(cx);

      exe.set_task_before_execution_hook(TRACING_TASK_INDEX, move |cx, _| {
        payload_read_back_bumper.reset(cx);
      });

      exe.set_task_after_execution_hook(TRACING_TASK_INDEX, move |cx, _| {
        payload_bumper.reset(cx);
      });

      let exe = GPUWaveFrontComputeRaytracingExecutor {
        graph_executor: exe,
        resource: trace_resource.clone(),
      };
      (current_hash, exe)
    });

    (exe, task_graph)
  }
}

#[derive(Clone)]
pub struct TraceTaskResource {
  pub payload_bumper: DeviceBumpAllocationInstance<u32>,
  pub payload_read_back_bumper: DeviceBumpAllocationInstance<u32>,
  pub info: Arc<TraceTaskMetaInfo>,
  pub current_sbt: StorageBufferReadonlyDataView<u32>,
  pub sbt_task_mapping: StorageBufferReadonlyDataView<SbtTaskMapping>,
  pub launch_size: StorageBufferReadonlyDataView<Vec3<u32>>,
}

fn create_task_graph<'a>(
  source: &GPURaytracingPipelineAndBindingSource,
  tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
  sbt_sys: ShaderBindingTableDeviceInfo,
  trace_resource: &'a mut Option<TraceTaskResource>,
  gpu: &GPU,
  size: u32,
) -> (DeviceTaskGraphBuildSource, &'a TraceTaskResource) {
  let mut graph = DeviceTaskGraphBuildSource::default();

  let device = &gpu.device;
  let trace_resource = trace_resource.get_or_insert_with(|| {
    let info = source.compute_trace_meta_info();

    let target_sbt_buffer = StorageBufferReadonlyDataView::create(device, &0);
    let sbt_task_mapping_buffer =
      StorageBufferReadonlyDataView::create(device, &info.create_sbt_mapping());
    // written in trace_ray. see RayLaunchSizeBuffer
    let launch_size_buffer = StorageBufferReadonlyDataView::create(device, &vec3(0, 0, 0));

    let payload_u32_len = size as usize * (info.payload_max_u32_count as usize);

    let buffer_allocator =
      MaybeCombinedStorageAllocator::new(gpu, "trace_ray user payload buffer", false, true);
    let a_a = MaybeCombinedAtomicU32StorageAllocator::new(
      gpu,
      "trace_ray user payload atomic buffer",
      false,
    );
    let payload_bumper =
      DeviceBumpAllocationInstance::new(payload_u32_len, device, &buffer_allocator, &a_a);
    let payload_read_back_bumper =
      DeviceBumpAllocationInstance::new(payload_u32_len, device, &buffer_allocator, &a_a);
    buffer_allocator.rebuild();
    a_a.rebuild();

    TraceTaskResource {
      payload_bumper,
      payload_read_back_bumper,
      info: Arc::new(info),
      current_sbt: target_sbt_buffer,
      sbt_task_mapping: sbt_task_mapping_buffer,
      launch_size: launch_size_buffer,
    }
  });

  let tracer_task = TraceTaskImpl {
    tlas_sys,
    sbt_sys,
    shared: trace_resource.clone(),
  };

  let mut ctx = AnyMap::default();
  ctx.register(TracingTaskSpawnerImplSource {
    payload_spawn_bumper: trace_resource.payload_bumper.clone(),
    payload_read_back: trace_resource.payload_read_back_bumper.clone(),
  });
  ctx.register(RayLaunchSizeBuffer {
    launch_size: trace_resource.launch_size.clone(),
  });

  // create core tracer task as almost every other task depend on this one
  let trace_task_id = graph.define_task_dyn(
    Box::new(OpaqueTaskWrapper(tracer_task)) as OpaqueTask,
    TraceTaskSelfPayload::sized_ty(),
    source.max_in_flight_trace_ray as usize,
  );
  assert_eq!(trace_task_id, 0);

  assert_eq!(source.ray_gen.len(), 1);
  let checker = &trace_resource.info;
  for s in &source.ray_gen {
    let task_id = graph.define_task_dyn(
      Box::new(OpaqueTaskWrapper(s.logic.build_device_future(&mut ctx))) as OpaqueTask,
      Vec3::<u32>::sized_ty(), // ignore the user defined payload(it's just a placeholder)
      s.max_in_flight.unwrap_or(1) as usize,
    );
    checker.assert_ray_gen_in_bound(task_id as usize);
  }

  for s in &source.closest_hit {
    let task_payload_ty = create_composite_task_payload_desc(
      graph.next_task_idx(),
      &s.user_defined_payload_input_ty,
      &RayClosestHitCtxPayload::sized_ty(),
    );
    let task_id = graph.define_task_dyn(
      Box::new(OpaqueTaskWrapper(s.logic.build_device_future(&mut ctx))) as OpaqueTask,
      task_payload_ty,
      s.max_in_flight.unwrap_or(1) as usize,
    );
    checker.assert_closest_hit_in_bound(task_id as usize);
  }

  for s in &source.miss_hit {
    let task_payload_ty = create_composite_task_payload_desc(
      graph.next_task_idx(),
      &s.user_defined_payload_input_ty,
      &RayMissHitCtxPayload::sized_ty(),
    );
    let task_id = graph.define_task_dyn(
      Box::new(OpaqueTaskWrapper(s.logic.build_device_future(&mut ctx))) as OpaqueTask,
      task_payload_ty,
      s.max_in_flight.unwrap_or(1) as usize,
    );
    checker.assert_miss_hit_in_bound(task_id as usize);
  }

  graph.capacity = size as usize;
  (graph, trace_resource)
}

impl GPURaytracingPipelineAndBindingSource {
  fn compute_trace_meta_info(&self) -> TraceTaskMetaInfo {
    let mut payload_max_u32_count = 0;

    // todo assert at least one for each stage will be defined
    let ray_gen_task_range_start = 1;
    let ray_gen_task_range_end = ray_gen_task_range_start + self.ray_gen.len();

    let closest_task_range_start = ray_gen_task_range_end;
    let closest_task_range_end = closest_task_range_start + self.closest_hit.len();
    let closest_tasks = self
      .closest_hit
      .iter()
      .enumerate()
      .map(|(i, s)| {
        let ty = &s.user_defined_payload_input_ty;
        payload_max_u32_count =
          payload_max_u32_count.max(ty.u32_size_count(StructLayoutTarget::Packed));
        ((i + closest_task_range_start) as u32, ty.clone())
      })
      .collect();

    let missing_task_start = closest_task_range_end;
    let missing_task_end = missing_task_start + self.miss_hit.len();
    let missing_tasks = self
      .miss_hit
      .iter()
      .enumerate()
      .map(|(i, s)| {
        let ty = &s.user_defined_payload_input_ty;
        payload_max_u32_count =
          payload_max_u32_count.max(ty.u32_size_count(StructLayoutTarget::Packed));
        ((i + missing_task_start) as u32, ty.clone())
      })
      .collect();

    TraceTaskMetaInfo {
      closest_tasks,
      missing_tasks,
      intersection_shaders: self.intersection.clone(),
      any_hit_shaders: self.any_hit.clone(),
      payload_max_u32_count,
      closest_task_range_start,
      missing_task_start,
      missing_task_end,
      ray_gen_task_idx: ray_gen_task_range_start as u32,
    }
  }
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
