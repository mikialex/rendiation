use crate::*;

pub struct ShaderFutureMap<F, T> {
  pub upstream: T,
  pub map: F,
}

impl<F, T, O> DeviceFuture for ShaderFutureMap<F, T>
where
  T: DeviceFuture,
  F: FnOnce(T::Output) -> O + Copy + 'static,
  O: ShaderAbstractRightValue + Default,
{
  type Output = O;
  type Invocation = ShaderFutureMapState<T::Invocation, F>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    ShaderFutureMapState {
      upstream: self.upstream.build_poll(ctx),
      map: self.map,
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn reset(&mut self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.upstream.reset(ctx, work_size)
  }
}

pub struct ShaderFutureMapState<T, F> {
  upstream: T,
  map: F,
}

impl<T, F, O> DeviceFutureInvocation for ShaderFutureMapState<T, F>
where
  T: DeviceFutureInvocation,
  F: FnOnce(T::Output) -> O + 'static + Copy,
  O: Default + ShaderAbstractRightValue,
{
  type Output = O;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<O> {
    let r = self.upstream.device_poll(ctx);
    let output = LocalLeftValueBuilder.create_left_value(O::default());
    if_by(r.is_ready, || {
      let o = (self.map)(r.payload);
      output.abstract_store(o);
    });

    (r.is_ready, output.abstract_load()).into()
  }
}
