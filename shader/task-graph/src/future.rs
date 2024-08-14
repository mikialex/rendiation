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

pub trait DeviceFuture {
  type State: 'static;
  type Output: Copy;
  fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State;
  fn required_poll_count(&self) -> usize;
  fn build_poll(
    &self,
    state: &Self::State,
    ctx: &mut DeviceTaskSystemBuildCtx,
  ) -> DevicePoll<Self::Output>;
  fn bind_input(&self, builder: &mut BindingBuilder);
}

pub trait DeviceFutureExt: Sized + DeviceFuture {
  fn map<F, O>(self, map: F) -> ShaderFutureMap<Self, F, O> {
    ShaderFutureMap {
      upstream: self,
      map,
      phantom: PhantomData,
    }
  }

  fn then<F, T>(self, then: F, then_instance: T) -> ShaderFutureThen<Self, F, T>
  where
    F: Fn(Self::Output) -> <T::State as ShaderAbstractLeftValue>::RightValue + Copy,
    T: DeviceFuture,
    T::State: ShaderAbstractLeftValue,
    T::Output: Default,
  {
    ShaderFutureThen {
      upstream: self,
      create_then_state_instance: then,
      then: then_instance,
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
  Output: Default + Copy,
{
  type State = ();
  type Output = Output;
  fn create_or_reconstruct_state(&self, _: &mut DynamicTypeBuilder) -> Self::State {}

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(
    &self,
    _: &Self::State,
    _: &mut DeviceTaskSystemBuildCtx,
  ) -> DevicePoll<Self::Output> {
    (val(true), Default::default()).into()
  }

  fn bind_input(&self, _: &mut BindingBuilder) {}
}

pub struct ShaderFutureMap<F, T, O> {
  pub upstream: F,
  pub map: T,
  pub phantom: PhantomData<O>,
}

impl<F, T, O> DeviceFuture for ShaderFutureMap<F, T, O>
where
  F: DeviceFuture,
  T: Fn(&F::Output) -> O + Copy,
  F::Output: Copy,
  O: ShaderAbstractRightValue + Default + Copy,
{
  type State = (F::State, BoxedShaderLoadStore<Node<bool>>);
  type Output = O;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count()
  }

  fn build_poll(
    &self,
    state: &Self::State,
    ctx: &mut DeviceTaskSystemBuildCtx,
  ) -> DevicePoll<Self::Output> {
    let (parent_state, upstream_resolved) = state;

    let output = O::default().into_local_left_value();
    if_by(upstream_resolved.abstract_load().not(), || {
      let r = self.upstream.build_poll(parent_state, ctx);
      if_by(r.is_ready, || {
        let o = (self.map)(&r.payload);
        output.abstract_store(o);
      });
    });

    (upstream_resolved.abstract_load(), output.abstract_load()).into()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State {
    (
      self.upstream.create_or_reconstruct_state(ctx),
      ctx.create_or_reconstruct_inline_state(false),
    )
  }
}

pub struct ShaderFutureThen<U, F, T> {
  pub upstream: U,
  pub create_then_state_instance: F,
  pub then: T,
}

pub struct ShaderFutureThenInstance<U, T> {
  upstream_state: U,
  upstream_resolved: BoxedShaderLoadStore<Node<bool>>,
  then_state: T,
  then_resolved: BoxedShaderLoadStore<Node<bool>>,
}

impl<U, F, T> DeviceFuture for ShaderFutureThen<U, F, T>
where
  U: DeviceFuture,
  F: Fn(U::Output) -> <T::State as ShaderAbstractLeftValue>::RightValue + Copy,
  T: DeviceFuture,
  T::State: ShaderAbstractLeftValue,
  T::Output: Default,
  T::Output: ShaderAbstractRightValue,
{
  type State = ShaderFutureThenInstance<U::State, T::State>;
  type Output = T::Output;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + self.then.required_poll_count()
  }

  fn build_poll(
    &self,
    state: &Self::State,
    ctx: &mut DeviceTaskSystemBuildCtx,
  ) -> DevicePoll<Self::Output> {
    let ShaderFutureThenInstance {
      upstream_state,
      upstream_resolved,
      then_state,
      then_resolved,
    } = state;

    if_by(upstream_resolved.abstract_load().not(), || {
      let r = self.upstream.build_poll(upstream_state, ctx);
      if_by(r.is_ready, || {
        upstream_resolved.abstract_store(val(true));
        let next = (self.create_then_state_instance)(r.payload);
        then_state.abstract_store(next);
      });
    });

    let output = T::Output::default().into_local_left_value();
    if_by(then_resolved.abstract_load(), || {
      let r = self.then.build_poll(then_state, ctx);
      if_by(r.is_ready, || {
        output.abstract_store(r.payload);
        then_resolved.abstract_store(val(true));
      });
    });

    (then_resolved.abstract_load(), output.abstract_load()).into()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State {
    ShaderFutureThenInstance {
      upstream_state: self.upstream.create_or_reconstruct_state(ctx),
      upstream_resolved: ctx.create_or_reconstruct_inline_state(false),
      then_state: self.then.create_or_reconstruct_state(ctx),
      then_resolved: ctx.create_or_reconstruct_inline_state(false),
    }
  }
}

pub struct TaskFuture<T, C>((usize, PhantomData<(T, C)>));

impl<T, C> DeviceFuture for TaskFuture<T, C>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type State = BoxedShaderLoadStore<Node<u32>>;
  type Output = Node<T>;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn create_or_reconstruct_state(&self, ctx: &mut DynamicTypeBuilder) -> Self::State {
    ctx.create_or_reconstruct_inline_state(u32::MAX)
  }

  fn build_poll(
    &self,
    state: &Self::State,
    ctx: &mut DeviceTaskSystemBuildCtx,
  ) -> DevicePoll<Self::Output> {
    let output = zeroed_val().into_local_left_value();

    ctx.poll_task::<T>(self.0 .0, state.abstract_load(), |r| {
      output.abstract_store(r);
      state.abstract_store(val(u32::MAX));
    });

    (
      state.abstract_load().equals(u32::MAX),
      output.abstract_load(),
    )
      .into()
  }

  fn bind_input(&self, _: &mut BindingBuilder) {}
}
