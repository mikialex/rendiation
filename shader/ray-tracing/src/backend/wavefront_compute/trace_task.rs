use parking_lot::RwLock;

use crate::*;

pub struct TraceTaskImpl {
  pub tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
  pub sbt_sys: ShaderBindingTableDeviceInfo,
  pub payload_bumper: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
  pub payload_read_back_bumper: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
  pub ray_info_bumper: DeviceBumpAllocationInstance<ShaderRayTraceCallStoragePayload>,
  pub info: Arc<TraceTaskMetaInfo>,
  pub current_sbt: StorageBufferReadOnlyDataView<u32>,
  pub sbt_task_mapping: StorageBufferReadOnlyDataView<SbtTaskMapping>,
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
        .read()
        .build_allocator_shader(ctx.compute_cx),
      current_sbt: ctx.compute_cx.bind_by(&self.current_sbt),
      sbt_task_mapping: ctx.compute_cx.bind_by(&self.sbt_task_mapping),
      downstream: AllDownStreamTasks { tasks },
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.tlas_sys.bind_pass(builder);
    self.sbt_sys.bind(builder);
    builder.bind(&self.payload_bumper.read().storage);
    self.payload_read_back_bumper.read().bind_allocator(builder);
    builder.bind(&self.current_sbt);
    builder.bind(&self.sbt_task_mapping);
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable>,
  sbt: ShaderBindingTableDeviceInfoInvocation,
  current_sbt: ReadOnlyStorageNode<u32>,
  sbt_task_mapping: ReadOnlyStorageNode<SbtTaskMapping>,
  info: Arc<TraceTaskMetaInfo>,
  untyped_payloads: StorageNode<[u32]>,
  payload_read_back_bumper: DeviceBumpAllocationInvocationInstance<u32>,
  downstream: AllDownStreamTasks,
}

const TASK_NOT_SPAWNED: u32 = u32::MAX;
const TASK_SPAWNED_FAILED: u32 = u32::MAX - 1;

#[derive(Clone)]
pub struct RayLaunchSizeBuffer {
  pub launch_size: StorageBufferReadOnlyDataView<Vec3<u32>>,
}
impl RayLaunchSizeBuffer {
  pub fn build(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> RayLaunchSizeInvocation {
    RayLaunchSizeInvocation {
      launch_size: ctx.compute_cx.bind_by(&self.launch_size),
    }
  }
  pub fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.launch_size);
  }
}
#[derive(Copy, Clone)]
pub struct RayLaunchSizeInvocation {
  launch_size: ReadOnlyStorageNode<Vec3<u32>>,
}
impl RayLaunchSizeInvocation {
  pub fn get(&self) -> Node<Vec3<u32>> {
    self.launch_size.load()
  }
}

#[derive(Copy, Clone)]
pub struct RayLaunchRawInfo {
  launch_id: Node<Vec3<u32>>,
  launch_size: Node<Vec3<u32>>,
}
impl RayLaunchRawInfo {
  pub fn new(linear_id: Node<u32>, launch_size: Node<Vec3<u32>>) -> Self {
    let launch_linear_idx = linear_id;
    let launch_x = launch_linear_idx % launch_size.x();
    let launch_linear_idx = launch_linear_idx / launch_size.x();
    let launch_y = launch_linear_idx % launch_size.y();
    let launch_linear_idx = launch_linear_idx / launch_size.y();
    let launch_z = launch_linear_idx % launch_size.z();
    Self {
      launch_id: (launch_x, launch_y, launch_z).into(),
      launch_size,
    }
  }
}
impl RayLaunchInfoProvider for RayLaunchRawInfo {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    self.launch_id
  }
  fn launch_size(&self) -> Node<Vec3<u32>> {
    self.launch_size
  }
}
impl RayGenCtxProvider for RayLaunchRawInfo {}

impl ShaderFutureInvocation for GPURayTraceTaskInvocationInstance {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    ctx.compute_cx.enable_log_shader();
    let trace_payload_all = ctx.access_self_payload::<TraceTaskSelfPayload>();

    let trace_payload_all_expand = trace_payload_all.load().expand();

    if_by(
      trace_payload_all_expand
        .sub_task_id
        .equals(TASK_NOT_SPAWNED),
      || {
        let trace_payload = trace_payload_all_expand.trace_call.expand();
        let sbt_task_mapping = self.sbt_task_mapping.load().expand();
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
            let r = val(ACCEPT_HIT).make_local_var();

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
          let closest_task_index = sbt_task_mapping.get_closest_task(closest_shader_index);
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

          let ty = TraceTaskSelfPayload::storage_node_sub_task_ty_field_ptr(trace_payload_all);
          ty.store(closest_task_index);
          let id = TraceTaskSelfPayload::storage_node_sub_task_id_field_ptr(trace_payload_all);
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
          let miss_task_index = sbt_task_mapping.get_miss_task(miss_sbt_index);
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

          let ty = TraceTaskSelfPayload::storage_node_sub_task_ty_field_ptr(trace_payload_all);
          ty.store(miss_task_index);
          let id = TraceTaskSelfPayload::storage_node_sub_task_id_field_ptr(trace_payload_all);
          id.store(sub_task_id);
        });
      },
    );

    let tid = trace_payload_all_expand.sub_task_ty;
    let id = trace_payload_all_expand.sub_task_id;

    let task_spawn_failed = trace_payload_all_expand
      .sub_task_id
      .equals(TASK_SPAWNED_FAILED);

    let trace_payload = TraceTaskSelfPayload::storage_node_trace_call_field_ptr(trace_payload_all);
    let trace_payload_idx =
      ShaderRayTraceCallStoragePayload::storage_node_payload_ref_field_ptr(trace_payload);

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

pub fn create_composite_task_payload_desc(
  task_id: u32,
  user_defined_payload_ty: &ShaderSizedValueType,
  ray_info_payload: &ShaderSizedValueType,
) -> ShaderSizedValueType {
  let mut combined_struct_desc =
    ShaderStructMetaInfo::new(&format!("task{}_payload_with_ray", task_id));
  combined_struct_desc.push_field_dyn("ray", ray_info_payload.clone());
  combined_struct_desc.push_field_dyn("payload", user_defined_payload_ty.clone());
  ShaderSizedValueType::Struct(combined_struct_desc)
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

  for (id, user_defined_payload_ty_desc) in task_range {
    switcher = switcher.case(*id, || {
      // copy untyped payload to typed specific tasks
      let payload =
        user_defined_payload_ty_desc.load_from_u32_buffer(untyped_payload_arr, untyped_payload_idx);

      let desc =
        create_composite_task_payload_desc(*id, user_defined_payload_ty_desc, ray_payload_desc);

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
      let if_resolved = cx.poll_task_dyn(*id as usize, task_id, |task_payload_node| {
        let (idx, _success) =
          bumper_read_back // todo, handle bump failed
            .bump_allocate_by(val(payload_ty_desc.u32_size_count()), |target, offset| {
              let user_defined_payload: StorageNode<AnyType> =
                unsafe { index_access_field(task_payload_node.handle(), 1) };

              payload_ty_desc.store_into_u32_buffer(user_defined_payload.load(), target, offset)
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
  pub(crate) payload_spawn_bumper: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
  pub(crate) payload_read_back: Arc<RwLock<DeviceBumpAllocationInstance<u32>>>,
}

impl TracingTaskSpawnerImplSource {
  pub fn create_invocation(
    &self,
    cx: &mut DeviceTaskSystemBuildCtx,
  ) -> TracingTaskSpawnerInvocation {
    TracingTaskSpawnerInvocation {
      payload_spawn_bumper: self
        .payload_spawn_bumper
        .read()
        .build_allocator_shader(cx.compute_cx),
      payload_read_back: self
        .payload_read_back
        .read()
        .build_allocator_shader(cx.compute_cx),
    }
  }

  pub fn bind(&self, builder: &mut BindingBuilder) {
    self.payload_spawn_bumper.read().bind_allocator(builder);
    self.payload_read_back.read().bind_allocator(builder);
  }
}

#[derive(Clone)]
pub(crate) struct TracingTaskSpawnerInvocation {
  pub(crate) payload_spawn_bumper: DeviceBumpAllocationInvocationInstance<u32>,
  pub(crate) payload_read_back: DeviceBumpAllocationInvocationInstance<u32>,
}

impl TracingTaskSpawnerInvocation {
  fn spawn_new_tracing_task(
    &mut self,
    task_group: &TaskGroupDeviceInvocationInstanceLateResolved,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
    parent: TaskParentRef,
    launch_id: Node<Vec3<u32>>,
    launch_size: Node<Vec3<u32>>,
  ) -> TaskFutureInvocationRightValue {
    let task_handle = val(u32::MAX).make_local_var();

    if_by(should_trace, || {
      let payload_size = payload_ty.u32_size_count();

      let (write_idx, success) =
        self
          .payload_spawn_bumper
          .bump_allocate_by(val(payload_size), |storage, write_idx| {
            payload_ty.store_into_u32_buffer(payload.into_node_untyped(), storage, write_idx);
          });

      if_by(success.not(), || {
        // todo, error report
      });

      let payload = ENode::<ShaderRayTraceCallStoragePayload> {
        launch_id,
        launch_size,
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

  fn read_back_user_payload(
    &mut self,
    payload_ty: ShaderSizedValueType,
    payload_ref: Node<u32>,
  ) -> ShaderNodeRawHandle {
    payload_ty
      .load_from_u32_buffer(self.payload_read_back.storage, payload_ref)
      .handle()
  }
}

pub const TRACING_TASK_INDEX: usize = 0;

impl<F, T, O, P> ShaderFutureProvider<(O, Node<P>)> for TraceNextRay<F, T>
where
  T: ShaderFutureProvider<O>,
  F: FnOnce(&O, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
  P: ShaderSizedValueNodeType + Default + Copy,
  O: ShaderAbstractRightValue + Default,
{
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<(O, Node<P>)> {
    let next_trace_logic = self.next_trace_logic;
    self
      .upstream
      .build_device_future(ctx)
      .then(
        move |o, then_invocation, cx| {
          let ctx = cx.invocation_registry.get_mut::<TracingCtx>().unwrap();
          let (should_trace, trace, payload) = next_trace_logic(&o, ctx);

          let (launch_id, launch_size) = if let Some(ray_gen) = ctx.ray_gen_ctx() {
            (ray_gen.launch_id(), ray_gen.launch_size())
          } else if let Some(closest) = ctx.closest_hit_ctx() {
            (closest.launch_id(), closest.launch_size())
          } else if let Some(missing) = ctx.miss_hit_ctx() {
            (missing.launch_id(), missing.launch_size())
          } else {
            unreachable!()
          };

          let parent = cx.generate_self_as_parent();
          cx.invocation_registry
            .get_mut::<TracingTaskSpawnerInvocation>()
            .unwrap()
            .spawn_new_tracing_task(
              &then_invocation.spawner,
              should_trace,
              trace,
              payload.handle(),
              P::sized_ty(),
              parent,
              launch_id,
              launch_size,
            )
        },
        TaskFuture::<TraceTaskSelfPayload>::new(TRACING_TASK_INDEX),
      )
      .map(move |(o, payload), cx| {
        let user_payload = cx
          .invocation_registry
          .get_mut::<TracingTaskSpawnerInvocation>()
          .unwrap()
          .read_back_user_payload(
            P::sized_ty(),
            payload.expand().trace_call.expand().payload_ref, // todo reduce unnecessary load
          );

        (o, unsafe { user_payload.into_node() })
      })
      .into_dyn()
  }
}
