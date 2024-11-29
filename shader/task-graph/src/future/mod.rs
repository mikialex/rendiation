use crate::*;

mod then;
pub use then::*;
mod map;
pub use map::*;
mod task;
pub use task::*;

#[derive(Clone, Copy)]
pub struct ShaderPoll<T> {
  pub result_state: Node<u32>,
  pub payload: T,
}

pub const SHADER_POLL_PENDING: u32 = 0;
pub const SHADER_POLL_RESOLVING: u32 = 1;
pub const SHADER_POLL_TERMINATED: u32 = 2;

impl<T> ShaderPoll<T> {
  pub fn is_resolving(&self) -> Node<bool> {
    self.result_state.equals(val(SHADER_POLL_RESOLVING))
  }

  pub fn resolving(&self, payload: T) -> Self {
    Self {
      result_state: val(SHADER_POLL_RESOLVING),
      payload,
    }
  }
}

impl<T> From<(Node<u32>, T)> for ShaderPoll<T> {
  fn from((result_state, payload): (Node<u32>, T)) -> Self {
    Self {
      result_state,
      payload,
    }
  }
}

pub trait ShaderFutureInvocation {
  type Output: 'static;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output>;
}

impl<T: 'static> ShaderFutureInvocation for Box<dyn ShaderFutureInvocation<Output = T>> {
  type Output = T;
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<T> {
    (**self).device_poll(ctx)
  }
}

pub struct DeviceReady<T>(pub T, pub BoxedShaderLoadStore<Node<bool>>);
impl<T: Copy + 'static> ShaderFutureInvocation for DeviceReady<T> {
  type Output = T;
  fn device_poll(&self, _: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<T> {
    let flag = val(SHADER_POLL_RESOLVING).make_local_var();
    if_by(self.1.abstract_load(), || {
      flag.abstract_store(val(SHADER_POLL_TERMINATED));
    });
    self.1.abstract_store(val(true));
    (flag.abstract_load(), self.0).into()
  }
}

pub trait ShaderFuture {
  type Output: 'static;
  type Invocation: ShaderFutureInvocation<Output = Self::Output> + 'static;

  fn required_poll_count(&self) -> usize;

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation;

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx);
}

pub type DynShaderFuture<T> =
  Box<dyn ShaderFuture<Output = T, Invocation = Box<dyn ShaderFutureInvocation<Output = T>>>>;

impl<O, I> ShaderFuture for Box<dyn ShaderFuture<Output = O, Invocation = I>>
where
  O: 'static,
  I: ShaderFutureInvocation<Output = O> + 'static,
{
  type Output = O;
  type Invocation = I;

  fn required_poll_count(&self) -> usize {
    (**self).required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    (**self).build_poll(ctx)
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    (**self).bind_input(builder)
  }
}

pub trait ShaderFutureExt: Sized + ShaderFuture + 'static {
  fn into_dyn(self) -> DynShaderFuture<Self::Output> {
    Box::new(WrapDynShaderFuture(self))
  }

  fn map<F, O>(self, map: F) -> ShaderFutureMap<F, Self>
  where
    F: FnOnce(Self::Output, &mut DeviceTaskSystemPollCtx) -> O + Copy + 'static,
    O: Default + ShaderAbstractRightValue,
  {
    ShaderFutureMap {
      upstream: self,
      map,
    }
  }

  fn then<F, T>(self, then_f: F, then: T) -> ShaderFutureThen<Self, F, T>
  where
    F: Fn(
        Self::Output,
        &T::Invocation,
        &mut DeviceTaskSystemPollCtx,
      ) -> <T::Invocation as ShaderAbstractLeftValue>::RightValue
      + Copy
      + 'static,
    Self::Output: ShaderAbstractRightValue,
    T: ShaderFuture,
    T::Invocation: ShaderAbstractLeftValue,
    T::Output: Default + ShaderAbstractRightValue,
  {
    ShaderFutureThen {
      upstream: self,
      create_then_invocation_instance: then_f,
      then,
    }
  }
  fn then_spawn_task<F, T>(
    self,
    then_f: F,
    task_ty: usize,
  ) -> ShaderFutureThen<Self, F, TaskFuture<T>> {
    ShaderFutureThen {
      upstream: self,
      create_then_invocation_instance: then_f,
      then: TaskFuture::new(task_ty),
    }
  }
}
impl<T: ShaderFuture + Sized + 'static> ShaderFutureExt for T {}

pub struct BaseShaderFuture<Output>(PhantomData<Output>);

impl<Output> Default for BaseShaderFuture<Output> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<Output> ShaderFuture for BaseShaderFuture<Output>
where
  Output: Default + Copy + 'static,
{
  type Output = Output;
  type Invocation = DeviceReady<Output>;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(&self, cx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    DeviceReady(Output::default(), cx.make_state::<Node<bool>>())
  }

  fn bind_input(&self, _: &mut DeviceTaskSystemBindCtx) {}
}

pub struct OpaqueTaskWrapper<T>(pub T);

impl<T: ShaderFuture> ShaderFuture for OpaqueTaskWrapper<T> {
  type Output = Box<dyn Any>;

  type Invocation = Box<dyn ShaderFutureInvocation<Output = Box<dyn Any>>>;

  fn required_poll_count(&self) -> usize {
    self.0.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    Box::new(OpaqueTaskInvocationWrapper(self.0.build_poll(ctx)))
      as Box<dyn ShaderFutureInvocation<Output = Box<dyn Any>>>
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.0.bind_input(builder)
  }
}

pub struct OpaqueTaskInvocationWrapper<T>(pub T);
impl<T: ShaderFutureInvocation> ShaderFutureInvocation for OpaqueTaskInvocationWrapper<T> {
  type Output = Box<dyn Any>;

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let p = self.0.device_poll(ctx);
    (p.result_state, Box::new(p.payload) as Box<dyn Any>).into()
  }
}

struct WrapDynShaderFuture<T>(T);
impl<T: ShaderFuture> ShaderFuture for WrapDynShaderFuture<T> {
  type Output = T::Output;
  type Invocation = Box<dyn ShaderFutureInvocation<Output = T::Output>>;

  fn required_poll_count(&self) -> usize {
    self.0.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    Box::new(self.0.build_poll(ctx))
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.0.bind_input(builder)
  }
}
