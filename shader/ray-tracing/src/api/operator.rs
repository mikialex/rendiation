use crate::*;

pub struct RaytracingFutureFromDeclaredPayloadInput<T>(T);

impl<T> NativeRayTracingShaderBuilder for RaytracingFutureFromDeclaredPayloadInput<T>
where
  T: NativeRayTracingShaderBuilder,
{
  type Ctx = T::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) {
    self.0.build(ctx);
    todo!()
  }
}

impl<T: ShaderFuture> ShaderFuture for RaytracingFutureFromDeclaredPayloadInput<T> {
  type State = T::State;
  type Output = T::Output;
  type Ctx = T::Ctx;
  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    self.0.reconstruct_state(ctx)
  }

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output> {
    // ctx.get_payload_input::<T>();
    todo!()
  }
}

pub struct RaytracingFutureTraceRay<F, T> {
  future: T,
  call: F,
}

impl<F, T> ShaderFuture for RaytracingFutureTraceRay<F, T>
where
  T: ShaderRayGenLogic,
  F: FnOnce() -> (Node<bool>, ShaderRayTraceCall) + Copy,
{
  type State = (T::State, BoxedShaderLoadStore<bool>);
  type Output = ShaderRayTraceCall;
  type Ctx = RayGenShaderCtx;
  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output> {
    let (parent_state, self_state) = state;
    let r = self.future.poll(parent_state, ctx);

    // if_by(r.is_ready.and(self_state.load()), || {
    //   (self.then)(ctx);
    //   self_state.store(val(true));
    // });
    // r

    // (r, ray)
    todo!()
  }

  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    (
      self.future.reconstruct_state(ctx),
      ctx.allocate_state::<bool>(),
    )
  }
}

impl<F, T> NativeRayTracingShaderBuilder for RaytracingFutureTraceRay<F, T>
where
  T: NativeRayTracingShaderBuilder,
  F: FnOnce() -> (Node<bool>, ShaderRayTraceCall) + Copy,
{
  type Ctx = T::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) {
    self.future.build(ctx);

    let (r, c) = (self.call)();
    if_by(r, || {
      // call native trace ray
    });
  }
}

pub struct ShaderFutureThen<F, T> {
  future: F,
  then: T,
}

impl<F, T> NativeRayTracingShaderBuilder for ShaderFutureThen<F, T>
where
  F: NativeRayTracingShaderBuilder,
  T: Fn(&F::Ctx) + Copy,
{
  type Ctx = F::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) {
    self.future.build(ctx);
    (self.then)(ctx);
  }
}

impl<F, T> ShaderFuture for ShaderFutureThen<F, T>
where
  F: ShaderFuture,
  T: Fn(&F::Ctx) + Copy,
  F::Output: Copy,
{
  type State = (F::State, LocalVarNode<bool>);
  type Output = F::Output;
  type Ctx = F::Ctx;

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output> {
    let (parent_state, self_state) = state;
    let r = self.future.poll(parent_state, ctx);

    if_by(r.is_ready.and(self_state.load()), || {
      (self.then)(ctx);
      self_state.store(val(true));
    });
    r
  }

  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    (
      self.future.reconstruct_state(ctx),
      val(false).make_local_var(),
    )
  }
}
