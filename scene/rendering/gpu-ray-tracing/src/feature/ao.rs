use crate::*;

pub struct RayTracingAORenderSystem {
  camera: DefaultRtxCameraRenderImplProvider,
  sbt: UpdateResultToken,
  scene_tlas: UpdateResultToken,
  rtx_system: Box<dyn GPURaytracingSystem>,
  rtx_device: Box<dyn GPURayTracingDeviceProvider>,
  rtx_acc: Box<dyn GPUAccelerationStructureSystemProvider>,
  executor: GPURaytracingPipelineExecutor,
}

impl RayTracingAORenderSystem {
  pub fn new(rtx: Box<dyn GPURaytracingSystem>) -> Self {
    let rtx_device = rtx.create_raytracing_device();
    Self {
      camera: Default::default(),
      scene_tlas: Default::default(),
      sbt: Default::default(),
      executor: rtx_device.create_raytracing_pipeline_executor(),
      rtx_acc: rtx.create_acceleration_structure_system(),
      rtx_device,
      rtx_system: rtx,
    }
  }
}

impl RenderImplProvider<SceneRayTracingAORenderer> for RayTracingAORenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.scene_tlas = source.register_reactive_query(scene_to_tlas(self.rtx_acc.clone()));

    // todo check mesh count grow
    let sbt = GPUSbt::new(self.rtx_device.create_sbt(2000, 2));
    let sbt = MultiUpdateContainer::new(sbt);
    // todo, add sbt maintain logic here
    // .with_source(source);

    self.sbt = source.register_multi_updater(sbt);
    self.camera.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.scene_tlas);
    source.deregister(&mut self.sbt);
    self.camera.deregister_resource(source);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> SceneRayTracingAORenderer {
    SceneRayTracingAORenderer {
      executor: self.executor.clone(),
      sbt: res
        .take_multi_updater_updated::<GPUSbt>(self.sbt)
        .unwrap()
        .target
        .clone(),
      scene_tlas: res.take_reactive_query_updated(self.scene_tlas).unwrap(),
      camera: self.camera.create_impl(res),
      rtx_system: self.rtx_system.clone(),
    }
  }
}

pub struct SceneRayTracingAORenderer {
  camera: Box<dyn RtxCameraRenderImpl>,
  executor: GPURaytracingPipelineExecutor,
  sbt: GPUSbt,
  rtx_system: Box<dyn GPURaytracingSystem>,
  scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlasHandle>,
}

impl SceneRayTracingAORenderer {
  pub fn render(
    &self,
    frame: &mut FrameCtx,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    ao_buffer: GPU2DTextureView,
  ) {
    let mut desc = GPURaytracingPipelineAndBindingSource::default();

    let camera = self.camera.get_rtx_camera(camera);

    let trace_base_builder = self.rtx_system.create_tracer_base_builder();
    let ray_gen_shader = RayTracingAOComputeTraceOperator {
      base: trace_base_builder.create_ray_gen_shader_base(),
      scene: self.scene_tlas.access(&scene).unwrap(),
      ao_buffer,
      max_sample_count: 8,
      camera,
    };

    desc.register_ray_gen::<u32>(ShaderFutureProviderIntoTraceOperator(ray_gen_shader));

    let mut rtx_encoder = self.rtx_system.create_raytracing_encoder();

    let canvas_size = frame.frame_size().into_u32();
    let sbt = self.sbt.inner.read();
    rtx_encoder.trace_ray(
      &desc,
      &self.executor,
      (canvas_size.0, canvas_size.1, 1),
      (*sbt).as_ref(),
    );
  }
}

#[derive(Clone)]
struct RayTracingAOComputeTraceOperator {
  base: Box<dyn TraceOperator<()>>,
  max_sample_count: u32,
  camera: Box<dyn RtxCameraRenderComponent>,
  scene: TlasHandle,
  ao_buffer: GPU2DTextureView,
}

impl ShaderHashProvider for RayTracingAOComputeTraceOperator {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}

impl ShaderFutureProvider for RayTracingAOComputeTraceOperator {
  type Output = ();
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    RayTracingAOComputeFuture {
      upstream: self.base.build_device_future(ctx),
      camera: self.camera.clone(),
      max_sample_count: self.max_sample_count,
      tracing: TracingFuture::default(),
    }
    .into_dyn()
  }
}

struct RayTracingAOComputeFuture {
  upstream: DynShaderFuture<()>,
  camera: Box<dyn RtxCameraRenderComponent>,
  max_sample_count: u32,
  tracing: TracingFuture<f32>,
}

impl ShaderFuture for RayTracingAOComputeFuture {
  type Output = ();

  type Invocation = RayTracingAOComputeInvocation;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + 1
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    RayTracingAOComputeInvocation {
      max_sample_count: self.max_sample_count,
      hit_position: ctx
        .state_builder
        .create_or_reconstruct_any_left_value_by_right::<Node<Vec3<f32>>>(),
      hit_normal: todo!(),
      hit_has_compute: todo!(),
      next_sample_idx: todo!(),
      occlusion_acc: todo!(),
      trace_on_the_fly: self.tracing.build_poll(ctx),
      camera: self.camera.build_invocation(ctx.compute_cx.bindgroups()),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.camera.bind(builder);
    todo!()
  }
}

struct RayTracingAOComputeInvocation {
  max_sample_count: u32,
  hit_position: BoxedShaderLoadStore<Node<Vec3<f32>>>,
  hit_normal: BoxedShaderLoadStore<Node<Vec3<f32>>>,
  hit_has_compute: BoxedShaderLoadStore<Node<bool>>,
  next_sample_idx: BoxedShaderLoadStore<Node<u32>>,
  occlusion_acc: BoxedShaderLoadStore<Node<f32>>,
  trace_on_the_fly: TracingFutureInvocation<f32>,
  camera: Box<dyn RtxCameraRenderInvocation>,
}

impl ShaderFutureInvocation for RayTracingAOComputeInvocation {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let current_idx = self.next_sample_idx.abstract_load();
    let sample_is_done = current_idx.greater_equal_than(self.max_sample_count);

    // self.camera.generate_ray(normalized_position);

    if_by(sample_is_done.not(), || {
      // let r = self.next_trace_on_the_fly.try_spawn_and_poll(ctx);
      // if_by(r.is_ready, || {
      //   self.next_sample_idx.abstract_store(current_idx + val(1));
      // });
      //
    });

    let occlusion = self.occlusion_acc.abstract_load() / val(self.max_sample_count as f32);
    (sample_is_done, ()).into()
  }
}
