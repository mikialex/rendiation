use crate::*;

impl<T, Output, Cx> NativeRayTracingShaderBuilder for BaseDeviceFuture<T, Output, Cx>
where
  T: NativeRayTracingShaderBuilder,
{
  type Ctx = T::Ctx;

  fn build(&self, _: &mut Self::Ctx) {}
}

pub struct TraceNextRay<F, T> {
  upstream: T,
  next_trace_logic: F,
}

impl<F, T> DeviceFuture for TraceNextRay<F, T>
where
  T: ShaderRayGenLogic,
  F: FnOnce() -> (Node<bool>, ShaderRayTraceCall) + Copy,
{
  type State = (T::State, BoxedShaderLoadStore<bool>);
  type Output = ShaderRayTraceCall;
  type Ctx = RayGenShaderCtx;
  fn poll(&self, state: &Self::State, ctx: &Self::Ctx) -> DevicePoll<Self::Output> {
    let (parent_state, self_state) = state;
    let r = self.upstream.poll(parent_state, ctx);

    // if_by(r.is_ready.and(self_state.load()), || {
    //   (self.then)(ctx);
    //   self_state.store(val(true));
    // });
    // r

    // (r, ray)
    todo!()
  }

  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    (
      self.upstream.create_or_reconstruct_state(ctx),
      ctx.allocate_state::<bool>(),
    )
  }
}

impl<F, T> NativeRayTracingShaderBuilder for TraceNextRay<F, T>
where
  T: NativeRayTracingShaderBuilder,
  T::Ctx: NativeRayTracingShaderCtx,
  F: FnOnce() -> (Node<bool>, ShaderRayTraceCall) + Copy,
{
  type Ctx = T::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) {
    self.upstream.build(ctx);

    let (r, c) = (self.next_trace_logic)();
    if_by(r, || {
      ctx.native_trace_ray(c);
    });
  }
}

impl<F, T, O> NativeRayTracingShaderBuilder for ShaderFutureMap<F, T, O>
where
  F: NativeRayTracingShaderBuilder,
  T: Fn(&F::Ctx) + Copy,
{
  type Ctx = F::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) {
    self.upstream.build(ctx);
    (self.map)(ctx);
  }
}
