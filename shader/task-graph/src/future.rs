use crate::*;

#[derive(Clone, Copy)]
pub struct DevicePoll<T> {
  pub is_ready: Node<bool>,
  pub payload: T,
}

impl<T> From<(Node<bool>, T)> for DevicePoll<T> {
  fn from((is_ready, payload): (Node<bool>, T)) -> Self {
    Self { is_ready, payload }
  }
}

pub trait DeviceFutureInvocation {
  type Output: 'static;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output>;
}

impl<T: 'static> DeviceFutureInvocation for Box<dyn DeviceFutureInvocation<Output = T>> {
  type Output = T;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<T> {
    (**self).device_poll(ctx)
  }
}

pub struct DeviceReady<T>(pub T);
impl<T: Copy + 'static> DeviceFutureInvocation for DeviceReady<T> {
  type Output = T;
  fn device_poll(&self, _: &mut DeviceTaskSystemPollCtx) -> DevicePoll<T> {
    (val(true), self.0).into()
  }
}

pub trait DeviceFuture {
  type Output: 'static;
  type Invocation: DeviceFutureInvocation<Output = Self::Output> + 'static;

  fn required_poll_count(&self) -> usize;

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation;

  fn bind_input(&self, builder: &mut BindingBuilder);

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32);
}

pub trait DeviceFutureExt: Sized + DeviceFuture {
  fn map<F, O>(self, map: F) -> ShaderFutureMap<F, Self, O> {
    ShaderFutureMap {
      upstream: self,
      map,
      phantom: PhantomData,
    }
  }

  fn then<F, T>(self, then_f: F, then: T) -> ShaderFutureThen<Self, F, T> {
    ShaderFutureThen {
      upstream: self,
      create_then_invocation_instance: then_f,
      then,
    }
  }
}
impl<T: DeviceFuture + Sized> DeviceFutureExt for T {}

pub struct BaseDeviceFuture<Output>(PhantomData<Output>);

impl<Output> Default for BaseDeviceFuture<Output> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<Output> DeviceFuture for BaseDeviceFuture<Output>
where
  Output: Default + Copy + 'static,
{
  type Output = Output;
  type Invocation = DeviceReady<Output>;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(&self, _: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    DeviceReady(Output::default())
  }

  fn bind_input(&self, _: &mut BindingBuilder) {}
  fn reset(&self, _: &mut DeviceParallelComputeCtx, _: u32) {}
}

pub struct ShaderFutureMap<F, T, O> {
  pub upstream: T,
  pub map: F,
  pub phantom: PhantomData<O>,
}

impl<F, T, O> DeviceFuture for ShaderFutureMap<F, T, O>
where
  T: DeviceFuture,
  F: Fn(T::Output) -> O + Copy + 'static,
  T::Output: Copy,
  O: ShaderAbstractRightValue + Default + Copy + 'static,
{
  type Output = O;
  type Invocation = ShaderFutureMapState<T::Invocation, O, F>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    ShaderFutureMapState {
      upstream: self.upstream.build_poll(ctx),
      map: self.map,
      phantom: PhantomData,
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.upstream.reset(ctx, work_size)
  }
}

pub struct ShaderFutureMapState<T, O, F> {
  upstream: T,
  map: F,
  phantom: PhantomData<O>,
}

impl<T, F, O> DeviceFutureInvocation for ShaderFutureMapState<T, O, F>
where
  T: DeviceFutureInvocation,
  F: Fn(T::Output) -> O,
  O: Default + ShaderAbstractRightValue + 'static,
{
  type Output = O;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<O> {
    let r = self.upstream.device_poll(ctx);
    let output = O::default().into_local_left_value();
    if_by(r.is_ready, || {
      let o = (self.map)(r.payload);
      output.abstract_store(o);
    });

    (r.is_ready, output.abstract_load()).into()
  }
}

pub struct ShaderFutureThen<U, F, T> {
  pub upstream: U,
  pub create_then_invocation_instance: F,
  pub then: T,
}

impl<U, F, T> DeviceFuture for ShaderFutureThen<U, F, T>
where
  U: DeviceFuture,
  T::Invocation: ShaderAbstractLeftValue,
  F: Fn(
      U::Output,
      &mut DeviceTaskSystemPollCtx,
    ) -> <T::Invocation as ShaderAbstractLeftValue>::RightValue
    + Copy
    + 'static,
  T: DeviceFuture,
  T::Output: Default + ShaderAbstractRightValue,
{
  type Output = T::Output;
  type Invocation = ShaderFutureThenInstance<U::Invocation, F, T::Invocation>;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + self.then.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    ShaderFutureThenInstance {
      upstream: self.upstream.build_poll(ctx),
      upstream_resolved: ctx.state_builder.create_or_reconstruct_inline_state(false),
      then: self.then.build_poll(ctx),
      create_then_invocation_instance: self.create_then_invocation_instance,
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder);
    self.then.bind_input(builder);
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.upstream.reset(ctx, work_size);
    self.then.reset(ctx, work_size)
  }
}

pub struct ShaderFutureThenInstance<U, F, T> {
  upstream: U,
  upstream_resolved: BoxedShaderLoadStore<Node<bool>>,
  create_then_invocation_instance: F,
  then: T,
}

impl<U, F, T> DeviceFutureInvocation for ShaderFutureThenInstance<U, F, T>
where
  U: DeviceFutureInvocation,
  T: DeviceFutureInvocation,
  T::Output: Default + ShaderAbstractRightValue,
  T: ShaderAbstractLeftValue,
  F: Fn(U::Output, &mut DeviceTaskSystemPollCtx) -> T::RightValue,
{
  type Output = T::Output;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<T::Output> {
    let ShaderFutureThenInstance {
      upstream,
      upstream_resolved,
      create_then_invocation_instance,
      then,
    } = self;

    if_by(upstream_resolved.abstract_load().not(), || {
      let r = upstream.device_poll(ctx);
      if_by(r.is_ready, || {
        upstream_resolved.abstract_store(val(true));
        let next = create_then_invocation_instance(r.payload, ctx);
        then.abstract_store(next);
      });
    });

    let resolved = val(false).into_local_left_value();
    let output = T::Output::default().into_local_left_value();
    if_by(upstream_resolved.abstract_load(), || {
      let r = self.then.device_poll(ctx);
      resolved.store(r.is_ready);
      output.abstract_store(r.payload);
    });

    (resolved.abstract_load(), output.abstract_load()).into()
  }
}

pub struct OpaqueTaskWrapper<T>(pub T);

impl<T: DeviceFuture> DeviceFuture for OpaqueTaskWrapper<T> {
  type Output = Box<dyn Any>;

  type Invocation = Box<dyn DeviceFutureInvocation<Output = Box<dyn Any>>>;

  fn required_poll_count(&self) -> usize {
    self.0.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    Box::new(OpaqueTaskInvocationWrapper(self.0.build_poll(ctx)))
      as Box<dyn DeviceFutureInvocation<Output = Box<dyn Any>>>
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.0.bind_input(builder)
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.0.reset(ctx, work_size)
  }
}

pub struct OpaqueTaskInvocationWrapper<T>(pub T);
impl<T: DeviceFutureInvocation> DeviceFutureInvocation for OpaqueTaskInvocationWrapper<T> {
  type Output = Box<dyn Any>;

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    let p = self.0.device_poll(ctx);
    (p.is_ready, Box::new(p.payload) as Box<dyn Any>).into()
  }
}

// pub struct TaskFuture<T, C>((usize, PhantomData<(T, C)>));

// impl<T, C> DeviceFuture for TaskFuture<T, C>
// where
//   T: ShaderSizedValueNodeType + Default + Copy,
// {
//   type State = BoxedShaderLoadStore<Node<u32>>;
//   type Output = Node<T>;

//   fn required_poll_count(&self) -> usize {
//     1
//   }

//   fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State {
//     ctx.create_or_reconstruct_inline_state(u32::MAX)
//   }

//   fn build_poll(
//     &self,
//     state: &Self::State,
//     ctx: &mut DeviceTaskSystemPollCtx,
//   ) -> DevicePoll<Self::Output> {
//     let output = zeroed_val().into_local_left_value();

//     ctx.poll_task::<T>(self.0 .0, state.abstract_load(), |r| {
//       output.abstract_store(r);
//       state.abstract_store(val(u32::MAX));
//     });

//     (
//       state.abstract_load().equals(u32::MAX),
//       output.abstract_load(),
//     )
//       .into()
//   }

//   fn bind_input(&self, _: &mut BindingBuilder) {}
//   fn reset(&self, _: &mut DeviceParallelComputeCtx, _: u32) {}
// }
