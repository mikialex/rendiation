use crate::*;

pub struct TraceTaskImpl {
  tlas_sys: Box<dyn GPUAccelerationStructureCompImplInstance>,
  sbt: ShaderBindingTableDeviceInfo,
  payload_bumper: DeviceBumpAllocationInstance<u32>,
  payload_read_back_bumper: DeviceBumpAllocationInstance<u32>,
  ray_info_bumper: DeviceBumpAllocationInstance<ShaderRayTraceCallStoragePayload>,
  info: Arc<TraceTaskMetaInfo>,
}

pub struct TraceTaskMetaInfo {
  closest_tasks: Vec<u32>,
  missing_tasks: Vec<u32>,
  intersection_shaders: Vec<Box<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  any_hit_shaders: Vec<Box<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
  payload_max_u32_count: u32,
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
      info: self.info.clone(),
      payload_read_back_bumper: self
        .payload_read_back_bumper
        .build_allocator_shader(ctx.compute_cx),
    }
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.tlas_sys.bind_pass(builder);
    self.sbt.bind(builder);
    builder.bind(&self.payload_bumper.storage);
    self.payload_read_back_bumper.bind_allocator(builder);
  }

  fn reset(&mut self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    self.payload_bumper = DeviceBumpAllocationInstance::new(
      (work_size * self.info.payload_max_u32_count) as usize,
      &ctx.gpu.device,
    );
    self.ray_info_bumper = DeviceBumpAllocationInstance::new(work_size as usize, &ctx.gpu.device);
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  tlas_sys: Box<dyn GPUAccelerationStructureCompImplInvocationTraversable>,
  sbt: ShaderBindingTableDeviceInfoInvocation,
  info: Arc<TraceTaskMetaInfo>,
  untyped_payloads: StorageNode<[u32]>,
  payload_read_back_bumper: DeviceBumpAllocationInvocationInstance<u32>,
}

const TASK_NOT_SPAWNED: u32 = u32::MAX;
const TASK_SPAWNED_FAILED: u32 = u32::MAX - 1;

impl DeviceFutureInvocation for GPURayTraceTaskInvocationInstance {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<Self::Output> {
    let trace_payload_all = ctx.access_self_payload::<TraceTaskSelfPayload>().load();

    let trace_payload_all_expand = trace_payload_all.expand();

    if_by(
      trace_payload_all_expand
        .sub_task_id
        .equals(TASK_NOT_SPAWNED),
      || {
        let trace_payload = trace_payload_all_expand.trace_call.expand();

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
            for (i, intersection_shader) in self.info.intersection_shaders.iter().enumerate() {
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
            for (i, any_hit_shader) in self.info.any_hit_shaders.iter().enumerate() {
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

          let closest_payload = ENode::<RayClosestHitCtxPayload> { hit_ctx: todo!() }.construct();

          let closest_shader_index = self.sbt.get_closest_handle(hit_group);
          let closest_task_index = closest_shader_index; // todo, make sure the shader index is task_index
          let sub_task_id = spawn_dynamic(
            &self.info.closest_tasks,
            ctx,
            closest_task_index,
            closest_payload.cast_untyped_node(),
            &RayClosestHitCtxPayload::sized_ty(),
            self.untyped_payloads,
            trace_payload.payload_ref,
            todo!(),
          );

          let ty: StorageNode<u32> = unsafe { index_access_field(trace_payload_all.handle(), 0) };
          ty.store(closest_task_index);
          let id: StorageNode<u32> = unsafe { index_access_field(trace_payload_all.handle(), 1) };
          id.store(sub_task_id);
        })
        .else_by(|| {
          let missing_payload = ENode::<RayMissHitCtxPayload> { hit_ctx: todo!() }.construct();

          let miss_sbt_index = self.sbt.get_missing_handle(trace_payload.miss_index);
          let miss_task_index = miss_sbt_index; // todo, make sure the shader index is task_index
          let sub_task_id = spawn_dynamic(
            &self.info.missing_tasks,
            ctx,
            miss_task_index,
            missing_payload.cast_untyped_node(),
            &RayMissHitCtxPayload::sized_ty(),
            self.untyped_payloads,
            trace_payload.payload_ref,
            todo!(),
          );

          let ty: StorageNode<u32> = unsafe { index_access_field(trace_payload_all.handle(), 0) };
          ty.store(miss_task_index);
          let id: StorageNode<u32> = unsafe { index_access_field(trace_payload_all.handle(), 1) };
          id.store(sub_task_id);
        });
      },
    );

    let tid = trace_payload_all_expand.sub_task_ty;
    let id = trace_payload_all_expand.sub_task_id;

    let task_spawn_failed = trace_payload_all_expand
      .sub_task_id
      .equals(TASK_SPAWNED_FAILED);

    let task_resolved: Node<bool> = todo!();

    (task_spawn_failed.or(task_resolved), ()).into()
  }
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct TraceTaskSelfPayload {
  pub sub_task_ty: u32,
  pub sub_task_id: u32,
  pub trace_call: ShaderRayTraceCallStoragePayload,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct ShaderRayTraceCallStoragePayload {
  pub tlas_idx: u32,
  pub ray_flags: u32,
  pub cull_mask: u32,
  pub sbt_ray_config_offset: u32,
  pub sbt_ray_config_stride: u32,
  pub miss_index: u32,
  pub ray_origin: Vec3<f32>,
  pub ray_direction: Vec3<f32>,
  pub range: Vec2<f32>,
  pub payload_ref: u32,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct RayClosestHitCtxPayload {
  pub hit_ctx: ShaderRayTraceCallStoragePayload,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct RayMissHitCtxPayload {
  pub hit_ctx: ShaderRayTraceCallStoragePayload,
}

#[must_use]
fn spawn_dynamic<'a>(
  task_range: impl IntoIterator<Item = &'a u32>,
  cx: &mut DeviceTaskSystemPollCtx,
  task_ty: Node<u32>,
  ray_payload: Node<AnyType>,
  ray_payload_desc: &ShaderSizedValueType,
  untyped_payload_arr: StorageNode<[u32]>,
  untyped_payload_idx: Node<u32>,
  untyped_task_payload_ty_desc: &ShaderSizedValueType,
) -> Node<u32> {
  let mut switcher = switch_by(task_ty);
  // if allocation failed, then task is directly resolved.
  // todo, should consider report this situation.
  let allocated_id = val(TASK_SPAWNED_FAILED).make_local_var();

  for &id in task_range {
    switcher = switcher.case(id, || {
      // copy untyped payload to typed specific tasks
      let payload =
        untyped_task_payload_ty_desc.load_from_u32_buffer(untyped_payload_arr, untyped_payload_idx);

      let mut combined_struct_desc =
        ShaderStructMetaInfo::new(&format!("task{}_payload_with_ray", id));
      combined_struct_desc.push_field_dyn("ray", ray_payload_desc.clone());
      combined_struct_desc.push_field_dyn("payload", untyped_task_payload_ty_desc.clone());
      let desc = ShaderSizedValueType::Struct(combined_struct_desc);

      let combined = ShaderNodeExpr::Compose {
        target: desc.clone(),
        parameters: vec![ray_payload.handle(), payload.handle()],
      }
      .insert_api();

      let re = cx.spawn_task_dyn(id as usize, combined, &desc).unwrap();

      allocated_id.store(re.task_handle);
    });
  }

  switcher.end_with_default(|| {});
  allocated_id.load()
}

// #[must_use]
fn poll_dynamic<'a>(
  task_range: impl IntoIterator<Item = &'a u32>,
  cx: &mut DeviceTaskSystemPollCtx,
  task_ty: Node<u32>,
  task_id: Node<u32>,
  ray_payload: Node<AnyType>,
  ray_payload_desc: &ShaderSizedValueType,
  bumper_read_back: DeviceBumpAllocationInvocationInstance<u32>,
  untyped_task_payload_ty_desc: &ShaderSizedValueType,
) -> Node<u32> {
  // let mut switcher = switch_by(task_ty);
  // // if allocation failed, then task is directly resolved.
  // // todo, should consider report this situation.
  // let allocated_id = val(TASK_SPAWNED_FAILED).make_local_var();

  // for &id in task_range {
  //   switcher = switcher.case(id, || {
  //     // copy untyped payload to typed specific tasks
  //     let payload =
  //       untyped_task_payload_ty_desc.load_from_u32_buffer(untyped_payload_arr, untyped_payload_idx);

  //     let mut combined_struct_desc =
  //       ShaderStructMetaInfo::new(&format!("task{}_payload_with_ray", id));
  //     combined_struct_desc.push_field_dyn("ray", ray_payload_desc.clone());
  //     combined_struct_desc.push_field_dyn("payload", untyped_task_payload_ty_desc.clone());
  //     let desc = ShaderSizedValueType::Struct(combined_struct_desc);

  //     let combined = ShaderNodeExpr::Compose {
  //       target: desc.clone(),
  //       parameters: vec![ray_payload.handle(), payload.handle()],
  //     }
  //     .insert_api();

  //     let re = cx.spawn_task_dyn(id as usize, combined, &desc).unwrap();

  //     allocated_id.store(re.task_handle);
  //   });
  // }

  // switcher.end_with_default(|| {});
  // allocated_id.load()

  todo!()
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
        // todo, error report
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
      }
      .construct();

      let payload = ENode::<TraceTaskSelfPayload> {
        sub_task_ty: val(u32::MAX),
        sub_task_id: val(TASK_NOT_SPAWNED),
        trace_call: payload,
      }
      .construct();

      let task = self.trace_task_spawner.spawn_new_task(payload).unwrap();

      task_handle.store(task);
    });

    TaskFutureInvocationRightValue {
      task_handle: task_handle.load(),
    }
  }
}
