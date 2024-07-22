use crate::*;

pub struct StackedStateMachine<T> {
  ctx_marker: PhantomData<T>,
  possible_states: Vec<Box<dyn Any>>,
}

pub struct StackedStateMachineInstance {
  trace_stack_state: WorkGroupSharedNode<[u8]>,
  trace_stack_info: WorkGroupSharedNode<[u32; 8]>,
}

impl<T> ShaderFuture for StackedStateMachine<T> {
  type State = StackedStateMachineInstance;
  type Output = ();
  type Ctx = T;
  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    todo!()
  }

  fn device_poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> (Node<bool>, Self::Output) {
    // ctx.get_payload_input::<T>();
    todo!()
  }
}

pub struct RaytracingFutureFromDeclaredPayloadInput<T>(T);

impl<T: ShaderFuture> ShaderFuture for RaytracingFutureFromDeclaredPayloadInput<T> {
  type State = T::State;
  type Output = T::Output;
  type Ctx = T::Ctx;
  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    self.0.reconstruct_state(ctx)
  }

  fn device_poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> (Node<bool>, Self::Output) {
    // ctx.get_payload_input::<T>();
    todo!()
  }
}

pub struct RaytracingFutureTraceRay<F, T> {
  future: F,
  payload_pass_in: Node<T>,
}

impl<F: ShaderRayGenLogic, T> ShaderFuture for RaytracingFutureTraceRay<F, T> {
  type State = F::State;
  type Output = ShaderRayTraceCall;
  type Ctx = RayGenShaderCtx;
  fn device_poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> (Node<bool>, Self::Output) {
    self.future.device_poll(state, ctx);
    // if_by return

    // ctx.trace_ray::<T>();

    todo!()
  }

  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    self.future.reconstruct_state(ctx)
  }
}

pub struct ShaderFutureThen<F, T> {
  future: F,
  then: T,
}

impl<F, T, U> ShaderFuture for ShaderFutureThen<F, T>
where
  F: ShaderFuture,
  T: Fn(F::Output) -> U,
{
  type State = F::State;
  type Output = U;
  type Ctx = F::Ctx;

  fn device_poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> (Node<bool>, Self::Output) {
    let (r, p) = self.future.device_poll(state, ctx);
    // r.select_branched(||{
    //   (val(true), (self.then)(p))
    // }, ||{
    //   (val(false), todo!())
    // })
    todo!()
  }

  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    self.future.reconstruct_state(ctx)
  }
}
