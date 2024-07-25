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
  pub future: F,
  pub map: T,
}

impl<F, T> DeviceFuture for ShaderFutureMap<F, T>
where
  F: DeviceFuture,
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
      (self.map)(ctx);
      self_state.store(val(true));
    });
    r
  }

  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    (
      self.future.create_or_reconstruct_state(ctx),
      val(false).make_local_var(),
    )
  }
}
