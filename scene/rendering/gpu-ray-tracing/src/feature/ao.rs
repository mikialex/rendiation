use crate::*;

pub struct SceneRayTracingAOFeature {
  executor: GPURaytracingPipelineExecutor,
  desc: GPURaytracingPipelineAndBindingSource,
  sbt: Box<dyn ShaderBindingTableProvider>,
  scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlasInstance>,
}

#[derive(Clone)]
struct RayTracingAOComputeTraceOperator {
  base: Box<dyn TraceOperator<()>>,
  max_sample_count: u32,
  scene: TlasInstance,
  ao_buffer: GPU2DTextureView,
}

impl ShaderHashProvider for RayTracingAOComputeTraceOperator {
  shader_hash_type_id! {}
}

impl NativeRayTracingShaderBuilder for RayTracingAOComputeTraceOperator {
  type Output = ();

  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> Self::Output {
    todo!()
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    todo!()
  }
}

impl ShaderFutureProvider for RayTracingAOComputeTraceOperator {
  type Output = ();
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    RayTracingAOComputeFuture {
      upstream: self.base.build_device_future(ctx),
      max_sample_count: self.max_sample_count,
      tracing: TracingFuture::default(),
    }
    .into_dyn()
  }
}

struct RayTracingAOComputeFuture {
  upstream: DynShaderFuture<()>,
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
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
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
}

impl ShaderFutureInvocation for RayTracingAOComputeInvocation {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let current_idx = self.next_sample_idx.abstract_load();
    let sample_is_done = current_idx.greater_equal_than(self.max_sample_count);

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

impl SceneRayTracingAOFeature {
  pub fn new(gpu: &GPU, tlas_size: Box<dyn Stream<Item = u32>>) -> Self {
    todo!()
  }

  pub fn render(
    &self,
    frame: &mut FrameCtx,
    system: Box<dyn GPURaytracingSystem>,
    previous_accumulation: GPU2DTextureView,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    ao_buffer: GPU2DTextureView,
  ) {
    let mut desc = GPURaytracingPipelineAndBindingSource::default();

    let trace_base_builder = system.create_tracer_base_builder();
    let ray_gen_shader = RayTracingAOComputeTraceOperator {
      base: trace_base_builder.create_ray_gen_shader_base(),
      scene: self.scene_tlas.access(&scene).unwrap(),
      ao_buffer,
      max_sample_count: 8,
    };

    desc.register_ray_gen::<u32>(ray_gen_shader);

    let mut rtx_encoder = system.create_raytracing_encoder();

    let canvas_size = frame.frame_size().into_u32();
    rtx_encoder.trace_ray(
      &desc,
      &self.executor,
      (canvas_size.0, canvas_size.1, 1),
      self.sbt.as_ref(),
    );
  }
}
