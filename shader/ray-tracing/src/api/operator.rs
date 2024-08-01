use crate::*;

impl<Output, Cx> NativeRayTracingShaderBuilder for BaseDeviceFuture<Output, Cx> {
  type Ctx = Cx;

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
  type State = (T::State, BoxedShaderLoadStore<Node<bool>>);
  type Output = ShaderRayTraceCall;
  type Ctx = RayGenShaderCtx;
  fn poll(
    &self,
    state: &Self::State,
    ccx: &mut ComputeCx,
    ctx: &mut DeviceTaskSystemBuildCtx,
    f_ctx: &mut Self::Ctx,
  ) -> DevicePoll<Self::Output> {
    let (parent_state, self_state) = state;
    let r = self.upstream.poll(parent_state, ccx, ctx, f_ctx);

    // if_by(r.is_ready.and(self_state.load()), || {
    //   (self.then)(ctx);
    //   self_state.store(val(true));
    // });
    // r

    // (r, ray)
    todo!()
  }

  fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State {
    (
      self.upstream.create_or_reconstruct_state(ctx),
      ctx.create_or_reconstruct_inline_state(false),
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

pub struct BoxState<T>(pub T);

impl<T: NativeRayTracingShaderBuilder> NativeRayTracingShaderBuilder for BoxState<T> {
  type Ctx = T::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) {
    self.0.build(ctx)
  }
}

impl<T> DeviceFuture for BoxState<T>
where
  T: DeviceFuture,
{
  type State = Box<dyn Any>;
  type Output = T::Output;
  type Ctx = T::Ctx;

  fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State {
    Box::new(self.0.create_or_reconstruct_state(ctx))
  }

  fn poll(
    &self,
    state: &Self::State,
    ccx: &mut ComputeCx,
    ctx: &mut DeviceTaskSystemBuildCtx,
    f_ctx: &mut Self::Ctx,
  ) -> DevicePoll<Self::Output> {
    let state = state.downcast_ref::<T::State>().unwrap();
    self.0.poll(state, ccx, ctx, f_ctx)
  }
}
