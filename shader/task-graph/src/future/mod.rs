use crate::*;

mod then;
pub use then::*;
mod map;
pub use map::*;
mod task;
pub use task::*;

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

pub type DynDeviceFuture<T> =
  Box<dyn DeviceFuture<Output = T, Invocation = Box<dyn DeviceFutureInvocation<Output = T>>>>;

impl<O, I> DeviceFuture for Box<dyn DeviceFuture<Output = O, Invocation = I>>
where
  O: 'static,
  I: DeviceFutureInvocation<Output = O> + 'static,
{
  type Output = O;
  type Invocation = I;

  fn required_poll_count(&self) -> usize {
    (**self).required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    (**self).build_poll(ctx)
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    (**self).bind_input(builder)
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    (**self).reset(ctx, work_size)
  }
}

pub trait DeviceFutureExt: Sized + DeviceFuture + 'static {
  fn into_dyn(self) -> DynDeviceFuture<Self::Output> {
    Box::new(WrapDynDeviceFuture(self))
  }

  fn map<F, O>(self, map: F) -> ShaderFutureMap<F, Self>
  where
    F: Fn(Self::Output) -> O,
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
        &mut DeviceTaskSystemPollCtx,
      ) -> <T::Invocation as ShaderAbstractLeftValue>::RightValue
      + Copy
      + 'static,
    Self::Output: ShaderAbstractRightValue,
    T: DeviceFuture,
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
impl<T: DeviceFuture + Sized + 'static> DeviceFutureExt for T {}

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

struct WrapDynDeviceFuture<T>(T);
impl<T: DeviceFuture> DeviceFuture for WrapDynDeviceFuture<T> {
  type Output = T::Output;
  type Invocation = Box<dyn DeviceFutureInvocation<Output = T::Output>>;

  fn required_poll_count(&self) -> usize {
    self.0.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    Box::new(self.0.build_poll(ctx))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.0.bind_input(builder)
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.0.reset(ctx, work_size)
  }
}
