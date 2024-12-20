use crate::*;

pub trait RayTracingCustomCtxProvider: ShaderHashProvider + 'static + Clone {
  type Invocation: Clone + 'static;
  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation;
  fn bind(&self, builder: &mut BindingBuilder);
}

#[derive(Clone)]
pub struct InjectCtx<T, C> {
  pub upstream: T,
  pub ctx: C,
}

impl<T: ShaderHashProvider + 'static, C: RayTracingCustomCtxProvider> ShaderHashProvider
  for InjectCtx<T, C>
{
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.upstream.hash_pipeline(hasher);
    self.ctx.hash_pipeline(hasher);
  }
}

impl<T, C> ShaderFutureProvider for InjectCtx<T, C>
where
  T::Output: 'static,
  T: ShaderFutureProvider,
  C: RayTracingCustomCtxProvider,
{
  type Output = T::Output;
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<T::Output> {
    InjectCtxShaderFuture {
      upstream: self.upstream.build_device_future(ctx),
      ctx: self.ctx.clone(),
    }
    .into_dyn()
  }
}

impl<T, C> NativeRayTracingShaderBuilder for InjectCtx<T, C>
where
  T: NativeRayTracingShaderBuilder,
  C: RayTracingCustomCtxProvider,
{
  type Output = T::Output;
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> Self::Output {
    self.ctx.build_invocation(ctx.binding_builder());
    self.upstream.build(ctx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.ctx.bind(builder);
  }
}

struct InjectCtxShaderFuture<T, C> {
  upstream: T,
  ctx: C,
}

impl<T, C> ShaderFuture for InjectCtxShaderFuture<T, C>
where
  T: ShaderFuture,
  C: RayTracingCustomCtxProvider,
{
  type Output = T::Output;

  type Invocation = InjectCtxShaderFutureInvocation<T::Invocation, C::Invocation>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    let upstream = self.upstream.build_poll(ctx);
    let invocation = self.ctx.build_invocation(ctx.compute_cx.bindgroups());
    InjectCtxShaderFutureInvocation {
      upstream,
      ctx: invocation,
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.upstream.bind_input(builder);
    self.ctx.bind(builder.binder);
  }
}

struct InjectCtxShaderFutureInvocation<T, C> {
  upstream: T,
  ctx: C,
}

impl<T, C> ShaderFutureInvocation for InjectCtxShaderFutureInvocation<T, C>
where
  T: ShaderFutureInvocation,
  C: Clone + 'static,
{
  type Output = T::Output;

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let r = self.upstream.device_poll(ctx);
    let t_ctx = ctx.invocation_registry.get_mut::<TracingCtx>().unwrap();
    t_ctx.registry.register(self.ctx.clone());
    r
  }
}
