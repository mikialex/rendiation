use crate::*;

pub struct ShaderFutureThen<U, F, T> {
  pub upstream: U,
  pub create_then_invocation_instance: F,
  pub then: T,
}

impl<U, F, T> DeviceFuture for ShaderFutureThen<U, F, T>
where
  U: DeviceFuture,
  U::Output: ShaderAbstractRightValue,
  F: Fn(
      U::Output,
      &T::Invocation,
      &mut DeviceTaskSystemPollCtx,
    ) -> <T::Invocation as ShaderAbstractLeftValue>::RightValue
    + Copy
    + 'static,
  T: DeviceFuture,
  T::Invocation: ShaderAbstractLeftValue,
  T::Output: Default + ShaderAbstractRightValue,
{
  type Output = (U::Output, T::Output);
  type Invocation = ShaderFutureThenInstance<U::Invocation, F, T::Invocation>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + self.then.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    ShaderFutureThenInstance {
      upstream: self.upstream.build_poll(ctx),
      upstream_resolved: ctx
        .state_builder
        .create_or_reconstruct_inline_state_with_default(false),
      upstream_output: ctx
        .state_builder
        .create_or_reconstruct_any_left_value_by_right::<U::Output>(),
      then: self.then.build_poll(ctx),
      create_then_invocation_instance: self.create_then_invocation_instance,
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.upstream.bind_input(builder);
    self.then.bind_input(builder);
  }

  fn reset(&mut self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.upstream.reset(ctx, work_size);
    self.then.reset(ctx, work_size)
  }
}

pub struct ShaderFutureThenInstance<U: DeviceFutureInvocation, F, T>
where
  U::Output: ShaderAbstractRightValue,
{
  upstream: U,
  upstream_output: <U::Output as ShaderAbstractRightValue>::AbstractLeftValue,
  upstream_resolved: BoxedShaderLoadStore<Node<bool>>,
  create_then_invocation_instance: F,
  then: T,
}

impl<U, F, T> DeviceFutureInvocation for ShaderFutureThenInstance<U, F, T>
where
  U: DeviceFutureInvocation,
  U::Output: ShaderAbstractRightValue,
  T: DeviceFutureInvocation,
  T::Output: Default + ShaderAbstractRightValue,
  T: ShaderAbstractLeftValue,
  F: Fn(U::Output, &T, &mut DeviceTaskSystemPollCtx) -> T::RightValue,
{
  type Output = (U::Output, T::Output);
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    let ShaderFutureThenInstance {
      upstream,
      upstream_resolved,
      upstream_output,
      create_then_invocation_instance,
      then,
    } = self;

    if_by(upstream_resolved.abstract_load().not(), || {
      let r = upstream.device_poll(ctx);
      if_by(r.is_ready, || {
        upstream_resolved.abstract_store(val(true));
        let next = create_then_invocation_instance(r.payload, &self.then, ctx);
        then.abstract_store(next);
      });
    });

    let resolved = LocalLeftValueBuilder.create_left_value(val(false));
    let output = LocalLeftValueBuilder.create_left_value(T::Output::default());
    if_by(upstream_resolved.abstract_load(), || {
      let r = self.then.device_poll(ctx);
      resolved.abstract_store(r.is_ready);
      output.abstract_store(r.payload);
    });

    (
      resolved.abstract_load(),
      (upstream_output.abstract_load(), output.abstract_load()),
    )
      .into()
  }
}
