use crate::*;

pub struct RaytracingFutureFromDeclaredPayloadInput<T>(T);

impl<T> RayTracingShaderBuilderWithNativeRayTracingSupport
  for RaytracingFutureFromDeclaredPayloadInput<T>
where
  T: RayTracingShaderBuilderWithNativeRayTracingSupport,
{
  type Ctx = T::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) -> Self {
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

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DeviceOption<Self::Output> {
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
  type State = T::State;
  type Output = ShaderRayTraceCall;
  type Ctx = RayGenShaderCtx;
  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DeviceOption<Self::Output> {
    let r = self.future.poll(state, ctx);
    // let ray = r.select_branched(|| (self.call)(), || todo!());

    // (r, ray)
    todo!()
  }

  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    self.future.reconstruct_state(ctx)
  }
}

impl<F, T> RayTracingShaderBuilderWithNativeRayTracingSupport for RaytracingFutureTraceRay<F, T>
where
  T: RayTracingShaderBuilderWithNativeRayTracingSupport,
  F: FnOnce() -> (Node<bool>, ShaderRayTraceCall) + Copy,
{
  type Ctx = T::Ctx;

  fn build(&self, ctx: &mut Self::Ctx) -> Self {
    self.future.build(ctx);

    let (r, c) = (self.call)();
    if_by(r, || {
      // call native trace ray
    });
    todo!()
  }
}

pub struct ShaderFutureThen<F, T> {
  future: F,
  then: T,
}

impl<F, T, U> ShaderFuture for ShaderFutureThen<F, T>
where
  F: ShaderFuture,
  T: Fn(F::Output) -> Node<U>,
  U: ShaderSizedValueNodeType,
  F::Output: Copy,
{
  type State = F::State;
  type Output = Node<U>;
  type Ctx = F::Ctx;

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DeviceOption<Self::Output> {
    let r = self.future.poll(state, ctx);

    // todo flag
    r.map(|v| (self.then)(v))
  }

  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    self.future.reconstruct_state(ctx)
  }
}
