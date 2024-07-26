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
  type State;
  type Output: Copy + ShaderAbstractRightValue;
  type Ctx: DeviceTaskSystemContextProvider;
  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State;
  fn poll(&self, state: &Self::State, ctx: &Self::Ctx) -> DevicePoll<Self::Output>;
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
    F: Fn(&Self::Ctx, Self::Output) -> <T::State as ShaderAbstractLeftValue>::RightValue + Copy,
    T: DeviceFuture<Ctx = Self::Ctx>,
    T::State: ShaderAbstractLeftValue,
    T::Output: Default,
  {
    ShaderFutureThen {
      upstream: self,
      create_then: then,
      then_instance,
    }
  }
}
impl<T: DeviceFuture + Sized> DeviceFutureExt for T {}

pub trait DeviceTaskSystemContextProvider {
  // todo, support PrimitiveShaderValueNodeType
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>>;

  fn read_write_task_payload<T>(&self) -> StorageNode<T>;

  /// argument must be valid for given task id to consume
  fn spawn_task<T>(&self, task_type: usize, argument: Node<T>) -> Node<u32>;
  fn poll_task<T>(
    &self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool>;
}

pub struct BaseDeviceFuture<T, Output, Cx>(PhantomData<(T, Output, Cx)>);

impl<T, Output, Cx> Default for BaseDeviceFuture<T, Output, Cx> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<T, Output, Cx> DeviceFuture for BaseDeviceFuture<T, Output, Cx>
where
  Output: Default + Copy + ShaderAbstractRightValue,
  Cx: DeviceTaskSystemContextProvider,
{
  type State = ();
  type Output = Output;
  type Ctx = Cx;
  fn create_or_reconstruct_state(&self, _: &mut Self::Ctx) -> Self::State {}

  fn poll(&self, _: &Self::State, _: &Self::Ctx) -> DevicePoll<Self::Output> {
    (val(true), Default::default()).into()
  }
}

pub struct ShaderFutureMap<F, T, O> {
  pub upstream: F,
  pub map: T,
  pub phantom: PhantomData<O>,
}

impl<F, T, O> DeviceFuture for ShaderFutureMap<F, T, O>
where
  F: DeviceFuture,
  T: Fn(&F::Ctx) -> O + Copy,
  F::Output: Copy,
  O: ShaderAbstractRightValue + Default + Copy,
{
  type State = (F::State, BoxedShaderLoadStore<Node<bool>>);
  type Output = O;
  type Ctx = F::Ctx;

  fn poll(&self, state: &Self::State, ctx: &Self::Ctx) -> DevicePoll<Self::Output> {
    let (parent_state, upstream_resolved) = state;

    let output = O::default().into_local_left_value();
    if_by(upstream_resolved.abstract_load().not(), || {
      let r = self.upstream.poll(parent_state, ctx);
      if_by(r.is_ready, || {
        let o = (self.map)(ctx);
        output.abstract_store(o);
      });
    });

    (upstream_resolved.abstract_load(), output.abstract_load()).into()
  }

  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    (
      self.upstream.create_or_reconstruct_state(ctx),
      ctx.create_or_reconstruct_inline_state(false),
    )
  }
}

pub struct ShaderFutureThen<U, F, T> {
  pub upstream: U,
  pub create_then: F,
  pub then_instance: T,
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
  F: Fn(&U::Ctx, U::Output) -> <T::State as ShaderAbstractLeftValue>::RightValue + Copy,
  T: DeviceFuture<Ctx = U::Ctx>,
  T::State: ShaderAbstractLeftValue,
  T::Output: Default,
{
  type State = ShaderFutureThenInstance<U::State, T::State>;
  type Output = T::Output;
  type Ctx = T::Ctx;

  fn poll(&self, state: &Self::State, ctx: &Self::Ctx) -> DevicePoll<Self::Output> {
    let ShaderFutureThenInstance {
      upstream_state,
      upstream_resolved,
      then_state,
      then_resolved,
    } = state;

    if_by(upstream_resolved.abstract_load().not(), || {
      let r = self.upstream.poll(upstream_state, ctx);
      if_by(r.is_ready, || {
        upstream_resolved.abstract_store(val(true));
        let next = (self.create_then)(ctx, r.payload);
        then_state.abstract_store(next);
      });
    });

    let output = T::Output::default().into_local_left_value();
    if_by(then_resolved.abstract_load(), || {
      let r = self.then_instance.poll(then_state, ctx);
      if_by(r.is_ready, || {
        output.abstract_store(r.payload);
        then_resolved.abstract_store(val(true));
      });
    });

    (then_resolved.abstract_load(), output.abstract_load()).into()
  }

  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    ShaderFutureThenInstance {
      upstream_state: self.upstream.create_or_reconstruct_state(ctx),
      upstream_resolved: ctx.create_or_reconstruct_inline_state(false),
      then_state: self.then_instance.create_or_reconstruct_state(ctx),
      then_resolved: ctx.create_or_reconstruct_inline_state(false),
    }
  }
}

pub struct TaskFuture<T, C>((usize, PhantomData<(T, C)>));

impl<T, C> DeviceFuture for TaskFuture<T, C>
where
  T: ShaderSizedValueNodeType + Default + Copy,
  C: DeviceTaskSystemContextProvider,
{
  type State = BoxedShaderLoadStore<Node<u32>>;
  type Output = Node<T>;
  type Ctx = C;

  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    ctx.create_or_reconstruct_inline_state(u32::MAX)
  }

  fn poll(&self, state: &Self::State, ctx: &Self::Ctx) -> DevicePoll<Self::Output> {
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
}
