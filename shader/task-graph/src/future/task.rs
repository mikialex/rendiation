use crate::*;

pub struct TaskFuture<T>(usize, PhantomData<T>);

impl<T> TaskFuture<T> {
  pub fn new(id: usize) -> Self {
    Self(id, PhantomData)
  }
}

impl<T> DeviceFuture for TaskFuture<T>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type Output = Node<T>;
  type Invocation = TaskFutureInvocation<T>;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    TaskFutureInvocation {
      task_ty: self.0,
      task_handle: ctx
        .state_builder
        .create_or_reconstruct_inline_state_with_default(u32::MAX),
      phantom: PhantomData,
    }
  }

  fn bind_input(&self, _: &mut BindingBuilder) {}
  fn reset(&self, _: &mut DeviceParallelComputeCtx, _: u32) {}
}

pub struct TaskFutureInvocation<T> {
  task_ty: usize,
  task_handle: BoxedShaderLoadStore<Node<u32>>,
  phantom: PhantomData<T>,
}

impl<T> DeviceFutureInvocation for TaskFutureInvocation<T>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type Output = Node<T>;

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    let output = LocalLeftValueBuilder.create_left_value(zeroed_val());

    ctx.poll_task::<T>(self.task_ty, self.task_handle.abstract_load(), |r| {
      output.abstract_store(r);
      self.task_handle.abstract_store(val(u32::MAX));
    });

    (
      self.task_handle.abstract_load().equals(u32::MAX),
      output.abstract_load(),
    )
      .into()
  }
}

pub struct TaskFutureInvocationRightValue {
  pub task_handle: Node<u32>,
}

impl<T> ShaderAbstractLeftValue for TaskFutureInvocation<T> {
  type RightValue = TaskFutureInvocationRightValue;

  fn abstract_load(&self) -> Self::RightValue {
    TaskFutureInvocationRightValue {
      task_handle: self.task_handle.abstract_load(),
    }
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.task_handle.abstract_store(payload.task_handle);
  }
}
