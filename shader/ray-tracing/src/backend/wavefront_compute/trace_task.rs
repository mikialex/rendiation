use crate::*;

pub struct TraceTaskImpl {
  tlas_sys: Box<dyn GPUAccelerationStructureCompImplInstance>,
  sbt: ShaderBindingTableDeviceInfo,
  payload_bumper: DeviceBumpAllocationInstance<u32>,
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
      sbt: self.sbt.build(ctx.compute_cx),
      untyped_payloads: ctx.compute_cx.bind_by(&self.payload_bumper.storage),
      closest_tasks: self.closest_tasks.clone(),
      missing_tasks: self.missing_tasks.clone(),
      intersection_shaders: todo!(),
      any_hit_shaders: todo!(),
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.tlas_sys.bind_pass(builder);
    self.sbt.bind(builder);
    self.payload_bumper.bind_allocator(builder);
  }

  fn reset(&self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    todo!()
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  tlas_sys: Box<dyn GPUAccelerationStructureCompImplInvocationTraversable>,
  sbt: ShaderBindingTableDeviceInfoInvocation,
  intersection_shaders: Vec<Box<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  any_hit_shaders: Vec<Box<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
  untyped_payloads: StorageNode<[u32]>,
  closest_tasks: Vec<u32>, // todo, ref
  missing_tasks: Vec<u32>,
}

impl DeviceFutureInvocation for GPURayTraceTaskInvocationInstance {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    let trace_payload = ctx
      .access_self_payload::<ShaderRayTraceCallStoragePayload>()
      .load()
      .expand();

    let ray_sbt_config = RaySBTConfig {
      offset: trace_payload.sbt_ray_config_offset,
      stride: trace_payload.sbt_ray_config_stride,
    };

    let closest_hit = self.tlas_sys.traverse(
      trace_payload,
      &|info, reporter| {
        let hit_group = info.hit_ctx.compute_sbt_hit_group(ray_sbt_config);
        let intersection_shader_index = self.sbt.get_intersection_handle(hit_group);
        let mut switcher = switch_by(intersection_shader_index);
        for (i, intersection_shader) in self.intersection_shaders.iter().enumerate() {
          switcher = switcher.case(i as u32, || {
            intersection_shader(info, reporter);
          });
        }
        switcher.end_with_default(|| {});
      },
      &|info| {
        let hit_group = info.hit_ctx.compute_sbt_hit_group(ray_sbt_config);
        let any_shader_index = self.sbt.get_any_handle(hit_group);
        let r = val(IGNORE_THIS_INTERSECTION).make_local_var();

        let mut switcher = switch_by(any_shader_index);
        for (i, any_hit_shader) in self.any_hit_shaders.iter().enumerate() {
          switcher = switcher.case(i as u32, || {
            r.store(any_hit_shader(info));
          });
        }
        switcher.end_with_default(|| {});
        r.load()
      },
    );

    if_by(closest_hit.is_some, || {
      let hit_group = closest_hit
        .payload
        .hit_ctx
        .compute_sbt_hit_group(ray_sbt_config);
      let closest_shader_index = self.sbt.get_closest_handle(hit_group);
      let closest_task_index = closest_shader_index; // todo, make sure the shader index is task_index
      spawn_dynamic(
        &self.closest_tasks,
        ctx,
        closest_task_index,
        todo!(),
        todo!(),
        todo!(),
      );
    })
    .else_by(|| {
      let miss_sbt_index = self.sbt.get_missing_handle(trace_payload.miss_index);
      let miss_task_index = miss_sbt_index; // todo, make sure the shader index is task_index
      spawn_dynamic(
        &self.missing_tasks,
        ctx,
        miss_task_index,
        todo!(),
        todo!(),
        todo!(),
      );
    });

    todo!()
  }
}

#[must_use]
fn spawn_dynamic<'a>(
  task_range: impl IntoIterator<Item = &'a u32>,
  cx: &mut DeviceTaskSystemPollCtx,
  task_ty: Node<u32>,
  ray_payload: Node<ShaderRayTraceCallStoragePayload>,
  untyped_payload_arr: StorageNode<u32>,
  untyped_task_payload_ty_desc: &ShaderSizedValueType,
) -> Node<u32> {
  let mut switcher = switch_by(task_ty);
  let allocated_id = val(u32::MAX).make_local_var(); // todo error handling

  for &id in task_range {
    switcher = switcher.case(id, || {
      // todo, copy untyped payload to typed specific tasks, update payload index
      let re = cx.spawn_task_dyn(id as usize, todo!(), todo!()).unwrap();
      allocated_id.store(re.task_handle);
    });
  }

  switcher.end_with_default(|| {});
  allocated_id.load()
}

struct TracingTaskSpawnerImpl {
  payload_bumper: DeviceBumpAllocationInvocationInstance<u32>,
  trace_task_spawner: TaskGroupDeviceInvocationInstance,
}

impl TracingTaskSpawner for TracingTaskSpawnerImpl {
  fn spawn_new_tracing_task(
    &mut self,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
  ) -> TaskFutureInvocationRightValue {
    let task_handle = val(u32::MAX).make_local_var();

    if_by(should_trace, || {
      let payload_size = payload_ty.u32_size_count();

      let (write_idx, success) =
        self
          .payload_bumper
          .bump_allocate_by(val(payload_size), |storage, write_idx| {
            payload_ty.store_into_u32_buffer(payload.into_node_untyped(), storage, write_idx);
          });

      if_by(success.not(), || {
        // todo, error handling
      });

      let payload = ENode::<ShaderRayTraceCallStoragePayload> {
        tlas_idx: trace_call.tlas_idx,
        ray_flags: trace_call.ray_flags,
        cull_mask: trace_call.cull_mask,
        sbt_ray_config_offset: trace_call.sbt_ray_config.offset,
        sbt_ray_config_stride: trace_call.sbt_ray_config.stride,
        miss_index: trace_call.miss_index,
        ray_origin: trace_call.ray.origin,
        ray_direction: trace_call.ray.direction,
        range: (trace_call.range.min, trace_call.range.max).into(),
        payload_ref: write_idx,
      };
      let task = self
        .trace_task_spawner
        .spawn_new_task(payload.construct())
        .unwrap();

      task_handle.store(task);
    });

    TaskFutureInvocationRightValue {
      task_handle: task_handle.load(),
    }
  }
}
