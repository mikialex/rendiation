use parking_lot::RwLock;

use crate::*;

pub struct TraceTaskImpl {
  pub tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
  pub sbt_sys: ShaderBindingTableDeviceInfo,
  pub payload_bumper: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
  pub payload_read_back_bumper: DeviceBumpAllocationInstance<u32>,
  pub ray_info_bumper: DeviceBumpAllocationInstance<ShaderRayTraceCallStoragePayload>,
  pub info: Arc<TraceTaskMetaInfo>,
  pub current_sbt: StorageBufferReadOnlyDataView<u32>,
}

pub struct TraceTaskMetaInfo {
  /// (task idx, payload ty desc)
  pub closest_tasks: Vec<(u32, ShaderSizedValueType)>,
  /// (task idx, payload ty desc)
  pub missing_tasks: Vec<(u32, ShaderSizedValueType)>,
  pub intersection_shaders: Vec<Arc<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  pub any_hit_shaders: Vec<Arc<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
  pub payload_max_u32_count: u32,
}

impl ShaderFuture for TraceTaskImpl {
  type Output = ();

  type Invocation = GPURayTraceTaskInvocationInstance;

  fn required_poll_count(&self) -> usize {
    2
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    let tasks = self
      .info
      .closest_tasks
      .iter()
      .map(|v| v.0)
      .chain(self.info.missing_tasks.iter().map(|v| v.0))
      .map(|id| id as usize)
      .map(|task_id| (task_id, ctx.get_or_create_task_group_instance(task_id)))
      .collect();

    GPURayTraceTaskInvocationInstance {
      tlas_sys: self.tlas_sys.build_shader(ctx.compute_cx),
      sbt: self.sbt_sys.build(ctx.compute_cx),
      untyped_payloads: ctx.compute_cx.bind_by(&self.payload_bumper.read().storage),
      info: self.info.clone(),
      payload_read_back_bumper: self
        .payload_read_back_bumper
        .build_allocator_shader(ctx.compute_cx),
      current_sbt: ctx.compute_cx.bind_by(&self.current_sbt),
      downstream: AllDownStreamTasks { tasks },
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.tlas_sys.bind_pass(builder);
    self.sbt_sys.bind(builder);
    builder.bind(&self.payload_bumper.read().storage);
    self.payload_read_back_bumper.bind_allocator(builder);
  }

  fn reset(&mut self, ctx: &mut DeviceParallelComputeCtx, work_size: u32) {
    *self.payload_bumper.write() = DeviceBumpAllocationInstance::new(
      (work_size * self.info.payload_max_u32_count) as usize,
      &ctx.gpu.device,
    );
    self.ray_info_bumper = DeviceBumpAllocationInstance::new(work_size as usize, &ctx.gpu.device);
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable>,
  sbt: ShaderBindingTableDeviceInfoInvocation,
  current_sbt: ReadOnlyStorageNode<u32>,
  info: Arc<TraceTaskMetaInfo>,
  untyped_payloads: StorageNode<[u32]>,
  payload_read_back_bumper: DeviceBumpAllocationInvocationInstance<u32>,
  downstream: AllDownStreamTasks,
}

const TASK_NOT_SPAWNED: u32 = u32::MAX;
const TASK_SPAWNED_FAILED: u32 = u32::MAX - 1;

impl ShaderFutureInvocation for GPURayTraceTaskInvocationInstance {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let trace_payload_all = ctx.access_self_payload::<TraceTaskSelfPayload>().load();

    let trace_payload_all_expand = trace_payload_all.expand();

    if_by(
      trace_payload_all_expand
        .sub_task_id
        .equals(TASK_NOT_SPAWNED),
      || {
        let trace_payload = trace_payload_all_expand.trace_call.expand();
        let current_sbt = self.current_sbt.load();

        let ray_sbt_config = RaySBTConfig {
          offset: trace_payload.sbt_ray_config_offset,
          stride: trace_payload.sbt_ray_config_stride,
        };

        let closest_hit = self.tlas_sys.traverse(
          trace_payload,
          &|info, reporter| {
            let hit_group = info.hit_ctx.compute_sbt_hit_group(ray_sbt_config);
            let intersection_shader_index =
              self.sbt.get_intersection_handle(current_sbt, hit_group);
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
            let any_shader_index = self.sbt.get_any_handle(current_sbt, hit_group);
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

          let closest_payload = ENode::<RayClosestHitCtxPayload> {
            ray_info: trace_payload_all_expand.trace_call,
            hit_ctx: hit_ctx_storage_from_hit_ctx(&closest_hit.payload.hit_ctx),
          }
          .construct();

          let closest_shader_index = self.sbt.get_closest_handle(current_sbt, hit_group);
          let closest_task_index = closest_shader_index; // todo, make sure the shader index is task_index
          let sub_task_id = spawn_dynamic(
            &self.info.closest_tasks,
            &self.downstream,
            closest_task_index,
            closest_payload.cast_untyped_node(),
            &RayClosestHitCtxPayload::sized_ty(),
            self.untyped_payloads,
            trace_payload.payload_ref,
            ctx.generate_self_as_parent(),
          );

          let ty: StorageNode<u32> = unsafe { index_access_field(trace_payload_all.handle(), 0) };
          ty.store(closest_task_index);
          let id: StorageNode<u32> = unsafe { index_access_field(trace_payload_all.handle(), 1) };
          id.store(sub_task_id);
        })
        .else_by(|| {
          let missing_payload = ENode::<RayMissHitCtxPayload> {
            ray_info: trace_payload_all_expand.trace_call,
          }
          .construct();

          let miss_sbt_index = self
            .sbt
            .get_missing_handle(current_sbt, trace_payload.miss_index);
          let miss_task_index = miss_sbt_index; // todo, make sure the shader index is task_index
          let sub_task_id = spawn_dynamic(
            &self.info.missing_tasks,
            &self.downstream,
            miss_task_index,
            missing_payload.cast_untyped_node(),
            &RayMissHitCtxPayload::sized_ty(),
            self.untyped_payloads,
            trace_payload.payload_ref,
            ctx.generate_self_as_parent(),
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

    let trace_payload: StorageNode<ShaderRayTraceCallStoragePayload> =
      unsafe { index_access_field(trace_payload_all.handle(), 2) };

    let trace_payload_idx: StorageNode<u32> =
      unsafe { index_access_field(trace_payload.handle(), 0) };

    let (missing_task_resolved, payload_idx) = poll_dynamic(
      &self.info.missing_tasks,
      &self.downstream,
      tid,
      id,
      &self.payload_read_back_bumper,
    );

    if_by(missing_task_resolved, || {
      trace_payload_idx.store(payload_idx);
    });

    let (closest_resolved, payload_idx) = poll_dynamic(
      &self.info.closest_tasks,
      &self.downstream,
      tid,
      id,
      &self.payload_read_back_bumper,
    );

    if_by(closest_resolved, || {
      trace_payload_idx.store(payload_idx);
    });

    (
      task_spawn_failed
        .or(missing_task_resolved)
        .or(closest_resolved),
      (),
    )
      .into()
  }
}

#[must_use]
fn spawn_dynamic<'a>(
  task_range: impl IntoIterator<Item = &'a (u32, ShaderSizedValueType)>,
  cx: &AllDownStreamTasks,
  task_ty: Node<u32>,
  ray_payload: Node<AnyType>,
  ray_payload_desc: &ShaderSizedValueType,
  untyped_payload_arr: StorageNode<[u32]>,
  untyped_payload_idx: Node<u32>,
  parent: TaskParentRef,
) -> Node<u32> {
  let mut switcher = switch_by(task_ty);
  // if allocation failed, then task is directly resolved.
  // todo, should consider report this situation.
  let allocated_id = val(TASK_SPAWNED_FAILED).make_local_var();

  for (id, payload_ty_desc) in task_range {
    switcher = switcher.case(*id, || {
      // copy untyped payload to typed specific tasks
      let payload = payload_ty_desc.load_from_u32_buffer(untyped_payload_arr, untyped_payload_idx);

      let mut combined_struct_desc =
        ShaderStructMetaInfo::new(&format!("task{}_payload_with_ray", id));
      combined_struct_desc.push_field_dyn("ray", ray_payload_desc.clone());
      combined_struct_desc.push_field_dyn("payload", payload_ty_desc.clone());
      let desc = ShaderSizedValueType::Struct(combined_struct_desc);

      let combined = ShaderNodeExpr::Compose {
        target: desc.clone(),
        parameters: vec![ray_payload.handle(), payload.handle()],
      }
      .insert_api();

      let re = cx.spawn_task_dyn(*id as usize, combined, &desc, parent);

      allocated_id.store(re);
    });
  }

  switcher.end_with_default(|| {});
  allocated_id.load()
}

struct AllDownStreamTasks {
  tasks: FastHashMap<usize, TaskGroupDeviceInvocationInstanceLateResolved>,
}

impl AllDownStreamTasks {
  pub fn poll_task_dyn(
    &self,
    task_tid: usize,
    task_id_node: Node<u32>,
    argument_read_back: impl FnOnce(StorageNode<AnyType>) + Copy,
  ) -> Node<bool> {
    self
      .tasks
      .get(&task_tid)
      .unwrap()
      .poll_task_dyn(task_id_node, argument_read_back)
  }

  pub fn spawn_task_dyn(
    &self,
    task_tid: usize,
    payload: Node<AnyType>,
    ty: &ShaderSizedValueType,
    parent: TaskParentRef,
  ) -> Node<u32> {
    self
      .tasks
      .get(&task_tid)
      .unwrap()
      .spawn_new_task_dyn(payload, parent, ty)
      .unwrap()
      .task_handle
  }
}

#[must_use]
fn poll_dynamic<'a>(
  task_range: impl IntoIterator<Item = &'a (u32, ShaderSizedValueType)>,
  cx: &AllDownStreamTasks,
  task_ty: Node<u32>,
  task_id: Node<u32>,
  bumper_read_back: &DeviceBumpAllocationInvocationInstance<u32>,
) -> (Node<bool>, Node<u32>) {
  let mut switcher = switch_by(task_ty);

  let resolved = val(false).make_local_var();
  let bump_read_position = val(u32::MAX).make_local_var();

  for (id, payload_ty_desc) in task_range {
    switcher = switcher.case(*id, || {
      let if_resolved = cx.poll_task_dyn(*id as usize, task_id, |node| {
        let (idx, _success) =
          bumper_read_back // todo, handle bump failed
            .bump_allocate_by(val(payload_ty_desc.u32_size_count()), |target, offset| {
              let payload: StorageNode<AnyType> = unsafe { index_access_field(node.handle(), 1) };

              payload_ty_desc.store_into_u32_buffer(payload.load(), target, offset)
            });
        bump_read_position.store(idx);
      });

      resolved.store(if_resolved);
    });
  }

  switcher.end_with_default(|| {});

  (resolved.load(), bump_read_position.load())
}

#[derive(Clone)]
pub(crate) struct TracingTaskSpawnerImplSource {
  pub(crate) payload_bumper: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
}

impl TracingTaskSpawnerImplSource {
  pub fn create_invocation(
    &self,
    cx: &mut DeviceTaskSystemBuildCtx,
  ) -> Box<dyn TracingTaskInvocationSpawner> {
    Box::new(TracingTaskSpawnerInvocationImpl {
      payload_bumper: self
        .payload_bumper
        .read()
        .build_allocator_shader(cx.compute_cx),
    })
  }

  pub fn bind(&self, builder: &mut BindingBuilder) {
    self.payload_bumper.read().bind_allocator(builder)
  }
}

#[derive(Clone)]
pub(crate) struct TracingTaskSpawnerInvocationImpl {
  pub(crate) payload_bumper: DeviceBumpAllocationInvocationInstance<u32>,
}

impl TracingTaskInvocationSpawner for TracingTaskSpawnerInvocationImpl {
  fn spawn_new_tracing_task(
    &mut self,
    task_group: &TaskGroupDeviceInvocationInstanceLateResolved,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
    parent: TaskParentRef,
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

      let task = task_group.spawn_new_task(payload, parent).unwrap();

      task_handle.store(task.task_handle);
    });

    TaskFutureInvocationRightValue {
      task_handle: task_handle.load(),
    }
  }
}
