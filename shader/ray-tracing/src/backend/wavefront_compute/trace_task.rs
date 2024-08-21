use dyn_clone::DynClone;

use crate::*;

pub struct TraceTaskImpl {
  payload_bumper: DeviceBumpAllocationInstance<u32>,
  tlas: Box<dyn GPUAccelerationStructureCompImplInstance>,
  closest_tasks: Vec<u32>,
  missing_tasks: Vec<u32>,
}

impl DeviceFuture for TraceTaskImpl {
  type Output = ();

  type Invocation = GPURayTraceTaskInvocationInstance;

  fn required_poll_count(&self) -> usize {
    todo!()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    GPURayTraceTaskInvocationInstance {
      acceleration_structure: todo!(),
      closest_tasks: self.closest_tasks.clone(),
      missing_tasks: self.missing_tasks.clone(),
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    todo!()
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    todo!()
  }
}

impl TracingTaskSpawner for TraceTaskImpl {
  fn spawn_new_tracing_task(
    &mut self,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
  ) -> TaskFutureInvocationRightValue {
    todo!()
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  acceleration_structure: Box<dyn GPUAccelerationStructureCompImplInvocationTraversable>,
  closest_tasks: Vec<u32>, // todo, ref
  missing_tasks: Vec<u32>,
}

impl DeviceFutureInvocation for GPURayTraceTaskInvocationInstance {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    todo!()
  }
}

fn spawn_dynamic<'a>(
  task_range: impl IntoIterator<Item = &'a u32>,
  cx: &mut DeviceTaskSystemPollCtx,
  task_ty: Node<u32>,
  payload: Node<AnyType>,
  task_payload_ty_desc: &ShaderSizedValueType,
) -> Node<u32> {
  let mut switcher = switch_by(task_ty);
  let allocated_id = val(u32::MAX).make_local_var(); // todo error handling

  for &id in task_range {
    switcher = switcher.case(id, || {
      let re = cx
        .spawn_task_dyn(id as usize, payload, task_payload_ty_desc)
        .unwrap();
      allocated_id.store(re.task_handle);
    });
  }

  switcher.end_with_default(|| {});
  allocated_id.load()
}

impl GPURayTraceTaskInvocationInstance {
  pub fn spawn_closest(
    &self,
    cx: &mut DeviceTaskSystemPollCtx,
    task_ty: Node<u32>,
    payload: Node<AnyType>,
    ty: &ShaderSizedValueType,
  ) -> Node<u32> {
    spawn_dynamic(&self.closest_tasks, cx, task_ty, payload, ty)
  }
  pub fn spawn_missing(
    &self,
    cx: &mut DeviceTaskSystemPollCtx,
    task_ty: Node<u32>,
    payload: Node<AnyType>,
    ty: &ShaderSizedValueType,
  ) -> Node<u32> {
    spawn_dynamic(&self.missing_tasks, cx, task_ty, payload, ty)
  }
}
