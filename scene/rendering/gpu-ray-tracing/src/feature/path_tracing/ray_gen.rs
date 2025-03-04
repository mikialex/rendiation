use anymap::AnyMap;

use super::*;

pub fn build_ray_gen_shader(
  base: &TraceFutureBaseBuilder,
  ctx: PTRayGenCtx,
) -> impl TraceOperator<()> + 'static {
  base.create_ray_gen_shader_base().inject_ctx(ctx)
}

struct PTRayGen {
  internal: Box<dyn TraceOperator<()>>,
}

impl ShaderFutureProvider for PTRayGen {
  type Output = ();

  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    todo!()
  }
}

struct PTRayGenShaderFuture {
  internal: Box<dyn TraceOperator<()>>,
  max_trace_depth: usize,
}
impl ShaderFuture for PTRayGenShaderFuture {
  type Output = ();

  type Invocation = PTRayGenShaderFutureInvocation;

  fn required_poll_count(&self) -> usize {
    // self.internal.
    todo!()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    PTRayGenShaderFutureInvocation {
      upstream: todo!(),
      current_flying_ray: todo!(),
      current_depth: ctx.make_state::<Node<u32>>(),
      current_throughput: ctx.make_state::<Node<Vec3<f32>>>(),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    todo!()
  }
}

struct PTRayGenShaderFutureInvocation {
  upstream: Box<dyn ShaderFutureInvocation<Output = ()>>,
  current_flying_ray: TracingFutureInvocation<CorePathPayload>,
  current_depth: BoxedShaderLoadStore<Node<u32>>,
  current_throughput: BoxedShaderLoadStore<Node<Vec3<f32>>>,
}

impl ShaderFutureInvocation for PTRayGenShaderFutureInvocation {
  type Output = ();
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let ray_origin = zeroed_val::<Vec3<f32>>().make_local_var();
    let ray_dir = zeroed_val::<Vec3<f32>>().make_local_var();
    let r = self.upstream.device_poll(ctx);

    let rt_ctx = ctx.invocation_registry.get_mut::<TracingCtx>().unwrap();
    let rg_cx = rt_ctx.expect_ray_gen_ctx();
    let image_position = rg_cx.launch_id().xy();
    let image_size = rg_cx.launch_size().xy();
    let cx = rt_ctx.expect_custom_cx::<PTRayGenCtxInvocation>();
    let radiance_buffer = cx.radiance_buffer;

    if_by(r.is_resolved(), || {
      // generate primary ray
      let normalized_position = image_position.into_f32() / image_size.into_f32();
      let ray = cx.camera.generate_ray(normalized_position);
      ray_origin.store(ray.origin);
      ray_dir.store(ray.direction);
    });

    let max_depth = cx.config.max_path_depth().load();
    let fly_ray = self.current_flying_ray.device_poll(ctx);
    if_by(fly_ray.is_resolved(), || {
      //
      let ENode::<CorePathPayload> {
        sampled_radiance,
        next_ray_origin,
        next_ray_dir,
        missed,
        pdf,
        brdf,
        normal,
      } = fly_ray.payload.expand();

      if_by(pdf.equals(0.), || {
        // mark this path as terminated
        self.current_depth.abstract_store(max_depth);
      });

      let throughput = self.current_throughput.abstract_load();
      let previous_r = radiance_buffer
        .load_storage_texture_texel(image_position)
        .xyz();
      if_by(missed.into_bool(), || {
        // mark this path as terminated
        self.current_depth.abstract_store(max_depth);
        let output_radiance = throughput * sampled_radiance + previous_r;
        radiance_buffer.write_texel(image_position, (output_radiance, val(1.)).into());
      })
      .else_by(|| {
        ray_origin.store(next_ray_origin);
        ray_dir.store(next_ray_dir);

        let cos = next_ray_dir.dot(normal);
        let throughput = throughput * cos * brdf / pdf.splat();
        self.current_throughput.abstract_store(throughput);

        let output_radiance = throughput * sampled_radiance + previous_r;
        radiance_buffer.write_texel(image_position, (output_radiance, val(1.)).into());
      });
    });

    storage_barrier();

    let current_depth = self.current_depth.abstract_load();
    let require_more_tracing = current_depth.less_than(max_depth);
    let should_spawn_ray_now = self
      .current_flying_ray
      .task_not_exist()
      .and(require_more_tracing);

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

    r.resolved.store(require_more_tracing.not());
    r
  }
}

#[derive(Clone)]
pub struct PTRayGenCtx {
  pub camera: Box<dyn RtxCameraRenderComponent>,
  pub radiance_buffer: StorageTextureViewReadWrite<GPU2DTextureView>,
  pub config: UniformBufferDataView<PTConfig>,
}
impl ShaderHashProvider for PTRayGenCtx {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}
impl RayTracingCustomCtxProvider for PTRayGenCtx {
  type Invocation = PTRayGenCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    PTRayGenCtxInvocation {
      camera: self.camera.build_invocation(cx),
      radiance_buffer: cx.bind_by(&self.radiance_buffer),
      config: cx.bind_by(&self.config),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.camera.bind(builder);
    builder.bind(&self.radiance_buffer);
    builder.bind(&self.config);
  }
}

#[derive(Clone)]
pub struct PTRayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  radiance_buffer: BindingNode<ShaderStorageTextureRW2D>,
  config: ShaderReadonlyPtrOf<PTConfig>,
}
