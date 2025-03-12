use anymap::AnyMap;
use rendiation_texture_gpu_process::ToneMapInvocation;

use super::*;

pub fn build_ray_gen_shader(
  base: &TraceFutureBaseBuilder,
  ctx: PTRayGenCtx,
  max_trace_depth: usize,
) -> impl TraceOperator<()> + 'static {
  PTRayGen {
    internal: Box::new(base.create_ray_gen_shader_base().inject_ctx(ctx)),
    max_trace_depth,
  }
}

#[derive(Clone)]
struct PTRayGen {
  internal: Box<dyn TraceOperator<()>>,
  max_trace_depth: usize,
}

impl ShaderHashProvider for PTRayGen {
  shader_hash_type_id! {}
}

impl NativeRayTracingShaderBuilder for PTRayGen {
  type Output = ();
  fn build(&self, _: &mut dyn NativeRayTracingShaderCtx) -> Self::Output {
    unimplemented!()
  }
  fn bind(&self, _: &mut BindingBuilder) {
    unimplemented!()
  }
}

impl ShaderFutureProvider for PTRayGen {
  type Output = ();

  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    PTRayGenShaderFuture {
      internal: self.internal.build_device_future(ctx),
      max_trace_depth: self.max_trace_depth,
    }
    .into_dyn()
  }
}

struct PTRayGenShaderFuture {
  internal: DynShaderFuture<()>,
  max_trace_depth: usize,
}
impl ShaderFuture for PTRayGenShaderFuture {
  type Output = ();

  type Invocation = PTRayGenShaderFutureInvocation;

  fn required_poll_count(&self) -> usize {
    self.internal.required_poll_count() + self.max_trace_depth
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    PTRayGenShaderFutureInvocation {
      upstream: self.internal.build_poll(ctx),
      current_flying_ray: TracingFuture::default().build_poll(ctx),
      current_depth: ctx.make_state::<Node<u32>>(),
      current_throughput: ctx
        .state_builder
        .create_or_reconstruct_inline_state_with_default(Vec3::one()),
      radiance: ctx.make_state::<Node<Vec3<f32>>>(),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.internal.bind_input(builder);
  }
}

struct PTRayGenShaderFutureInvocation {
  upstream: Box<dyn ShaderFutureInvocation<Output = ()>>,
  current_flying_ray: TracingFutureInvocation<CorePathPayload>,
  current_depth: BoxedShaderLoadStore<Node<u32>>,
  current_throughput: BoxedShaderLoadStore<Node<Vec3<f32>>>,
  radiance: BoxedShaderLoadStore<Node<Vec3<f32>>>,
}

impl ShaderFutureInvocation for PTRayGenShaderFutureInvocation {
  type Output = ();
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let ray_origin = zeroed_val::<Vec3<f32>>().make_local_var();
    let ray_dir = zeroed_val::<Vec3<f32>>().make_local_var();
    let r = self.upstream.device_poll(ctx);

    let rt_ctx = ctx.invocation_registry.get::<TracingCtx>().unwrap();
    let rg_cx = rt_ctx.expect_ray_gen_ctx();
    let image_position = rg_cx.launch_id().xy();
    let image_size = rg_cx.launch_size().xy();
    let cx = rt_ctx.expect_custom_cx::<PTRayGenCtxInvocation>();
    let result_buffer = cx.result_buffer;
    let tonemap = cx.tonemap.clone();
    let sample_count = cx.config.current_sample_count().load().into_f32();

    if_by(r.is_resolved(), || {
      // generate primary ray
      let normalized_position = image_position.into_f32() / image_size.into_f32();
      let ray = cx.camera.generate_ray(normalized_position);
      ray_origin.store(ray.origin);
      ray_dir.store(ray.direction);
    });

    let max_depth = cx.config.max_path_depth().load();
    let current_depth = self.current_depth.abstract_load().make_local_var();
    let radiance = self.radiance.abstract_load().make_local_var();
    let fly_ray = self.current_flying_ray.device_poll(ctx);
    if_by(fly_ray.is_resolved(), || {
      let ENode::<CorePathPayload> {
        sampled_radiance,
        surface_radiance,
        next_ray_origin,
        next_ray_dir,
        missed,
        pdf,
        brdf,
        normal,
      } = fly_ray.payload.expand();

      if_by(pdf.equals(0.), || {
        // mark this path as terminated
        current_depth.store(max_depth);
      });

      let throughput = self.current_throughput.abstract_load();
      if_by(missed.into_bool(), || {
        // mark this path as terminated
        current_depth.store(max_depth);
        let output_radiance = throughput * sampled_radiance + surface_radiance + radiance.load();
        radiance.store(output_radiance);
        self.radiance.abstract_store(output_radiance);
      })
      .else_by(|| {
        current_depth.store(current_depth.load() + val(1));
        ray_origin.store(next_ray_origin);
        ray_dir.store(next_ray_dir);

        let cos = next_ray_dir.dot(normal);
        let pdf = pdf.max(0.0001);
        let throughput = throughput * cos * brdf / pdf.splat();
        self.current_throughput.abstract_store(throughput);

        let output_radiance = throughput * sampled_radiance + radiance.load();
        radiance.store(output_radiance);
        self.radiance.abstract_store(output_radiance);
      });
    });

    storage_barrier();

    let current_depth = current_depth.load();
    self.current_depth.abstract_store(current_depth);
    let require_more_tracing = current_depth.less_than(max_depth);
    let should_spawn_ray_now = self
      .current_flying_ray
      .task_not_exist()
      .and(require_more_tracing)
      .and(ctx.is_fallback_task().not());

    let trace_call = ShaderRayTraceCall {
      tlas_idx: val(0), // only one tlas, select first
      ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
      cull_mask: val(u32::MAX),
      sbt_ray_config: PTRayType::Core.to_sbt_cfg(),
      miss_index: val(0),
      ray: ShaderRay {
        origin: ray_origin.load(),
        direction: ray_dir.load(),
      },
      range: ShaderRayRange::default(),
    };

    let new_trace_ray = ctx.spawn_new_tracing_task(
      should_spawn_ray_now,
      trace_call,
      zeroed_val(),
      &self.current_flying_ray,
    );
    self.current_flying_ray.abstract_store(new_trace_ray);

    let final_resolved = require_more_tracing.not();
    if_by(final_resolved, || {
      let averaged_result = result_buffer
        .load_storage_texture_texel(image_position)
        .xyz();

      let sample_radiance = radiance.load();
      let ldr_result = tonemap.compute_ldr(sample_radiance);

      // we not enable this is to see if anything cause nan besides for 0 pdf
      // let is_nan = sample_result
      //   .x()
      //   .is_nan()
      //   .or(sample_result.y().is_nan())
      //   .or(sample_result.z().is_nan());
      // let sample_result = is_nan.select(averaged_result, ldr_result);

      let updated_average =
        (averaged_result * sample_count + ldr_result) / (sample_count + val(1.)).splat();

      result_buffer.write_texel(image_position, (updated_average, val(1.)).into());
    });

    r.resolved.store(final_resolved);
    r
  }
}

#[derive(Clone)]
pub struct PTRayGenCtx {
  pub camera: Box<dyn RtxCameraRenderComponent>,
  pub result_buffer: StorageTextureViewReadWrite<GPU2DTextureView>,
  pub config: UniformBufferDataView<PTConfig>,
  pub tonemap: ToneMap,
}
impl ShaderHashProvider for PTRayGenCtx {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
    self.tonemap.hash_pipeline(hasher);
  }
}
impl RayTracingCustomCtxProvider for PTRayGenCtx {
  type Invocation = PTRayGenCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    PTRayGenCtxInvocation {
      camera: self.camera.build_invocation(cx),
      result_buffer: cx.bind_by(&self.result_buffer),
      config: cx.bind_by(&self.config),
      tonemap: self.tonemap.build(cx),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.camera.bind(builder);
    builder.bind(&self.result_buffer);
    builder.bind(&self.config);
    self.tonemap.bind(builder);
  }
}

#[derive(Clone)]
pub struct PTRayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  result_buffer: BindingNode<ShaderStorageTextureRW2D>,
  config: ShaderReadonlyPtrOf<PTConfig>,
  tonemap: ToneMapInvocation,
}
