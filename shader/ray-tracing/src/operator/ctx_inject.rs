use crate::*;

pub trait RayTracingCustomCtxProvider: ShaderHashProvider + 'static + Clone {
  type Invocation: Clone + 'static;
  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation;
  fn bind(&self, builder: &mut BindingBuilder);
}

pub struct InjectCtx<T, C> {
  pub upstream: T,
  pub ctx: C,
}

impl<X, T, C> ShaderFutureProvider<X> for InjectCtx<T, C>
where
  X: 'static,
  T: ShaderFutureProvider<X>,
  C: RayTracingCustomCtxProvider,
{
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<X> {
    InjectCtxShaderFuture {
      upstream: self.upstream.build_device_future(ctx),
      ctx: self.ctx.clone(),
    }
    .into_dyn()
  }
}

impl<O, T, C> NativeRayTracingShaderBuilder<O> for InjectCtx<T, C>
where
  T: NativeRayTracingShaderBuilder<O>,
  C: RayTracingCustomCtxProvider,
{
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O {
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
    let invocation = self.ctx.build_invocation(ctx.compute_cx.bindgroups());
    InjectCtxShaderFutureInvocation {
      upstream: self.upstream.build_poll(ctx),
      ctx: invocation,
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.ctx.bind(builder.binder);
    self.upstream.bind_input(builder);
  }

  fn reset(&mut self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.upstream.reset(ctx, work_size);
    // todo, resize self sized managed resource?
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
    let t_ctx = ctx.invocation_registry.get_mut::<TracingCtx>().unwrap();
    t_ctx.registry.register(self.ctx.clone());
    self.upstream.device_poll(ctx)
  }
}
