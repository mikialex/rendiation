use crate::*;

pub struct TraceTaskImpl {
  payload_bumper: DeviceBumpAllocationInstance<u32>,
  tlas_sys: Box<dyn GPUAccelerationStructureCompImplInstance>,
  closest_tasks: Vec<u32>,
  missing_tasks: Vec<u32>,
}

impl DeviceFuture for TraceTaskImpl {
  type Output = ();

  type Invocation = GPURayTraceTaskInvocationInstance;

  fn required_poll_count(&self) -> usize {
    2
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    GPURayTraceTaskInvocationInstance {
      tlas_sys: self.tlas_sys.build_shader(ctx.compute_cx),
      untyped_payloads: ctx.compute_cx.bind_by(&self.payload_bumper.storage),
      closest_tasks: self.closest_tasks.clone(),
      missing_tasks: self.missing_tasks.clone(),
      intersection_shaders: todo!(),
      any_hit_shaders: todo!(),
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.tlas_sys.bind_pass(builder)
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    todo!()
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  tlas_sys: Box<dyn GPUAccelerationStructureCompImplInvocationTraversable>,
  intersection_shaders: Vec<Box<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  any_hit_shaders: Vec<Box<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
  untyped_payloads: StorageNode<[u32]>,
  closest_tasks: Vec<u32>, // todo, ref
  missing_tasks: Vec<u32>,
}

impl DeviceFutureInvocation for GPURayTraceTaskInvocationInstance {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    let trace_payload = ctx.access_self_payload::<TracePayload>().load().expand();

    let closest_hit = self.tlas_sys.traverse(
      trace_payload,
      &|info, reporter| {
        //
      },
      &|info| {
        //
        todo!()
      },
    );

    if_by(closest_hit.is_some, || {
      // dispatch closest task
      // spawn_dynamic
    })
    .else_by(|| {
      // dispatch missing task
      // spawn_dynamic
    });

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

struct TracingTaskSpawnerImpl {
  payload_bumper: DeviceBumpAllocationInstance<u32>,
}

impl TracingTaskSpawner for TracingTaskSpawnerImpl {
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
