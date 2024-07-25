use crate::*;

#[derive(Clone, Copy)]
pub struct DevicePoll<T> {
  pub is_ready: Node<bool>,
  pub payload: T,
}

#[derive(Clone, Copy)]
pub struct DeviceOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}

// impl<T> From<(Node<bool>, T)> for DeviceOption<T> {
//   fn from((is_some, payload): (Node<bool>, T)) -> Self {
//     Self { is_some, payload }
//   }
// }

// impl<T: Copy> DeviceOption<T> {
//   pub fn some(payload: T) -> Self {
//     Self {
//       is_some: val(true),
//       payload,
//     }
//   }

//   pub fn map<U: ShaderSizedValueNodeType>(
//     self,
//     f: impl FnOnce(T) -> Node<U> + Copy,
//   ) -> DeviceOption<Node<U>> {
//     let u = zeroed_val().make_local_var();
//     if_by(self.is_some, || u.store(f(self.payload)));
//     (self.is_some, u.load()).into()
//   }
//   pub fn map_none<U: ShaderSizedValueNodeType>(
//     self,
//     f: impl FnOnce(T) -> Node<U> + Copy,
//   ) -> DeviceOption<Node<U>> {
//     let u = zeroed_val().make_local_var();
//     if_by(self.is_some.not(), || u.store(f(self.payload)));
//     (self.is_some, u.load()).into()
//   }
// }

pub trait DeviceFuture {
  type State;
  type Output;
  type Ctx: DeviceTaskSystemContextProvider;
  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State;
  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output>;
}

pub trait DeviceTaskSystemContextProvider {
  // todo, support PrimitiveShaderValueNodeType
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>>;

  fn read_write_task_payload<T>(&self) -> StorageNode<T>;

  /// argument must be valid for given task id to consume
  fn spawn_task<T>(&mut self, task_type: usize, argument: Node<T>) -> Node<u32>;
  fn poll_task<T>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool>;
}

pub struct BaseDeviceFuture<T, Output, Cx>(PhantomData<(T, Output, Cx)>);

impl<T, Output: Default, Cx: DeviceTaskSystemContextProvider> DeviceFuture
  for BaseDeviceFuture<T, Output, Cx>
{
  type State = ();
  type Output = Output;
  type Ctx = Cx;
  fn create_or_reconstruct_state(&self, _: &mut Self::Ctx) -> Self::State {}

  fn poll(&self, _: &Self::State, _: &mut Self::Ctx) -> DevicePoll<Self::Output> {
    DevicePoll {
      is_ready: val(true),
      payload: Default::default(),
    }
  }
}

pub struct ShaderFutureMap<F, T> {
  pub upstream: F,
  pub map: T,
}

impl<F, T> DeviceFuture for ShaderFutureMap<F, T>
where
  F: DeviceFuture,
  T: Fn(&F::Ctx) + Copy,
  F::Output: Copy,
{
  type State = (F::State, BoxedShaderLoadStore<Node<bool>>);
  type Output = F::Output;
  type Ctx = F::Ctx;

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output> {
    let (parent_state, upstream_resolved) = state;

    // let output = todo!();
    // if_by(upstream_resolved.abstract_load().not(), || {
    //   let r = self.upstream.poll(parent_state, ctx);
    //   if_by(r.is_ready, || {
    //     (self.map)(ctx);
    //     output.store(r.payload);
    //   })
    // });

    // (upstream_resolved.abstract_load(), output);
    todo!()
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
  pub then: F,
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
  F: Fn(&U::Ctx, U::Output) -> <T::State as ShaderAbstractLoadStore>::Value + Copy,
  T: DeviceFuture<Ctx = U::Ctx>,
  T::State: ShaderAbstractLoadStore,
  T::Output: Default,
{
  type State = ShaderFutureThenInstance<U::State, T::State>;
  type Output = T::Output;
  type Ctx = T::Ctx;

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output> {
    let ShaderFutureThenInstance {
      upstream_state,
      upstream_resolved,
      then_state,
      then_resolved,
    } = state;

    // if_by(upstream_resolved.abstract_load().not(), || {
    //   let r = self.upstream.poll(upstream_state, ctx);
    //   upstream_resolved.abstract_store(val(true));
    //   if_by(r.is_ready, || {
    //     let next = (self.then)(ctx, r.payload);
    //     then_state.abstract_store((self.then)(ctx, r.payload));
    //   });
    // });

    // let output = todo!();
    // if_by(upstream_resolved.abstract_load(), || {
    //   let r = self.then_instance.poll(then_state, ctx);
    //   if_by(r.is_ready, || {
    //     output.store(r.payload);
    //     then_resolved.abstract_store(val(true));
    //   });
    // });

    // (then_resolved.abstract_load(), output)
    todo!()
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
