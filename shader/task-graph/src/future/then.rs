use crate::*;

/// the uninitialized then instance must return pending result when polling
pub struct ShaderFutureThen<U, F, T> {
  pub upstream: U,
  pub create_then_invocation_instance: F,
  pub then: T,
}

impl<U, F, T> ShaderFuture for ShaderFutureThen<U, F, T>
where
  U: ShaderFuture,
  U::Output: ShaderAbstractRightValue,
  F: Fn(
      U::Output,
      &T::Invocation,
      &mut DeviceTaskSystemPollCtx,
    ) -> <T::Invocation as ShaderAbstractLeftValue>::RightValue
    + Copy
    + 'static,
  T: ShaderFuture,
  T::Invocation: ShaderAbstractLeftValue,
{
  type Output = (U::Output, T::Output);
  type Invocation = ShaderFutureThenInstance<U::Invocation, F, T::Invocation>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + self.then.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    ShaderFutureThenInstance {
      upstream: self.upstream.build_poll(ctx),
      then: self.then.build_poll(ctx),
      create_then_invocation_instance: self.create_then_invocation_instance,
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.upstream.bind_input(builder);
    self.then.bind_input(builder);
  }
}

/// the uninitialized then instance must return pending result when polling
pub struct ShaderFutureThenInstance<U: ShaderFutureInvocation, F, T>
where
  U::Output: ShaderAbstractRightValue,
{
  upstream: U,
  create_then_invocation_instance: F,
  then: T,
}

impl<U, F, T> ShaderFutureInvocation for ShaderFutureThenInstance<U, F, T>
where
  U: ShaderFutureInvocation,
  U::Output: ShaderAbstractRightValue,
  T: ShaderFutureInvocation,
  T: ShaderAbstractLeftValue,
  F: Fn(U::Output, &T, &mut DeviceTaskSystemPollCtx) -> T::RightValue,
{
  type Output = (U::Output, T::Output);
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let ShaderFutureThenInstance {
      upstream,
      create_then_invocation_instance,
      then,
    } = self;

    let r = upstream.device_poll(ctx);

    if_by(r.is_resolved(), || {
      let next = create_then_invocation_instance(r.payload.clone(), &self.then, ctx);
      then.abstract_store(next);
    });

    storage_barrier();

    let rr = self.then.device_poll(ctx);

    (rr.resolved, (r.payload, rr.payload)).into()
  }
}
