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
    todo!()
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    todo!()
  }
}

struct PTRayGenShaderFutureInvocation {}

impl ShaderFutureInvocation for PTRayGenShaderFutureInvocation {
  type Output = ();
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    todo!()
  }
}

#[derive(Clone)]
struct PTRayGenCtx {
  camera: Box<dyn RtxCameraRenderComponent>,
  radiance_buffer: StorageTextureViewReadWrite<GPU2DTextureView>,
  config: UniformBufferDataView<PTConfig>,
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
struct PTRayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  radiance_buffer: BindingNode<ShaderStorageTextureRW2D>,
  config: ShaderReadonlyPtrOf<PTConfig>,
}
