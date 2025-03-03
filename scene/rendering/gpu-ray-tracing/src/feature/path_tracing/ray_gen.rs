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
    let r = self.upstream.device_poll(ctx);
    if_by(r.is_resolved(), || {
      //
    });

    let rt_ctx = ctx.invocation_registry.get_mut::<TracingCtx>().unwrap();
    let cx = rt_ctx.expect_custom_cx::<PTRayGenCtxInvocation>();

    let max_depth = cx.config.max_path_depth().load();
    let fly_ray = self.current_flying_ray.device_poll(ctx);
    if_by(fly_ray.is_resolved(), || {
      //
      let ENode::<CorePathPayload> {
        sampled_radiance,
        next_ray_origin,
        next_ray_dir,
        missed,
      } = fly_ray.payload.expand();

      if_by(missed.into_bool(), || {
        // mark this path as terminated
        self.current_depth.abstract_store(max_depth);
      });
    });

    // self.current_flying_ray.abstract_load();

    storage_barrier();

    let current_depth = self.current_depth.abstract_load();
    let require_more_tracing = current_depth.less_than(max_depth);

    let new_trace_ray = ctx.spawn_new_tracing_task(
      require_more_tracing,
      todo!(),
      todo!(),
      &self.current_flying_ray,
    );
    self.current_flying_ray.abstract_store(new_trace_ray);

    todo!()
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
