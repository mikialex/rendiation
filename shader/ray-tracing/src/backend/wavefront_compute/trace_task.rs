use crate::*;

pub struct TraceTaskImpl {
  pub tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
  pub sbt_sys: ShaderBindingTableDeviceInfo,
  pub shared: TraceTaskResource,
}

pub struct TraceTaskMetaInfo {
  pub ray_gen_task_idx: u32,
  /// (task idx, payload ty desc)
  pub closest_tasks: Vec<(u32, ShaderSizedValueType)>,
  /// (task idx, payload ty desc)
  pub missing_tasks: Vec<(u32, ShaderSizedValueType)>,
  pub intersection_shaders: Vec<Arc<dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter)>>,
  pub any_hit_shaders: Vec<Arc<dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>>>,
  pub payload_max_u32_count: u32,

  pub closest_task_range_start: usize,
  pub missing_task_start: usize,
  pub missing_task_end: usize,
}

impl TraceTaskMetaInfo {
  pub fn assert_ray_gen_in_bound(&self, task_id: usize) {
    assert!((1..self.closest_task_range_start).contains(&task_id));
  }

  pub fn assert_closest_hit_in_bound(&self, task_id: usize) {
    assert!((self.closest_task_range_start..self.missing_task_start).contains(&task_id));
  }

  pub fn assert_miss_hit_in_bound(&self, task_id: usize) {
    assert!((self.missing_task_start..self.missing_task_end).contains(&task_id));
  }

  pub fn create_sbt_mapping(&self) -> SbtTaskMapping {
    SbtTaskMapping::new(
      1, // todo, remove?
      self.closest_task_range_start as u32,
      self.missing_task_start as u32,
    )
  }
}

impl ShaderFuture for TraceTaskImpl {
  type Output = ();

  type Invocation = GPURayTraceTaskInvocationInstance;

  fn required_poll_count(&self) -> usize {
    2
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    let resource = &self.shared;
    let tasks = resource
      .info
      .closest_tasks
      .iter()
      .map(|v| v.0)
      .chain(resource.info.missing_tasks.iter().map(|v| v.0))
      .map(|id| id as usize)
      .map(|task_id| (task_id, ctx.get_or_create_task_group_instance(task_id)))
      .collect();

    GPURayTraceTaskInvocationInstance {
      tlas_sys: self.tlas_sys.build_shader(ctx.compute_cx),
      sbt: self.sbt_sys.build(ctx.compute_cx),
      untyped_payloads: ctx
        .compute_cx
        .bind_abstract_storage(&resource.payload_bumper.storage),
      info: resource.info.clone(),
      payload_read_back_bumper: resource
        .payload_read_back_bumper
        .build_allocator_shader(ctx.compute_cx),
      current_sbt: ctx.compute_cx.bind_by(&resource.current_sbt),
      sbt_task_mapping: ctx.compute_cx.bind_by(&resource.sbt_task_mapping),
      downstream: AllDownStreamTasks { tasks },
      has_terminated: ctx.make_state::<Node<Bool>>(),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.tlas_sys.bind_pass(builder);
    self.sbt_sys.bind(builder);

    let resource = &self.shared;
    builder
      .binder
      .bind_abstract_storage(&resource.payload_bumper.storage);
    resource.payload_read_back_bumper.bind_allocator(builder);
    builder.bind(&resource.current_sbt);
    builder.bind(&resource.sbt_task_mapping);
  }
}

pub struct GPURayTraceTaskInvocationInstance {
  tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable>,
  sbt: ShaderBindingTableDeviceInfoInvocation,
  current_sbt: ShaderReadonlyPtrOf<u32>,
  sbt_task_mapping: ShaderReadonlyPtrOf<SbtTaskMapping>,
  info: Arc<TraceTaskMetaInfo>,
  untyped_payloads: ShaderPtrOf<[u32]>,
  payload_read_back_bumper: DeviceBumpAllocationInvocationInstance<u32>,
  downstream: AllDownStreamTasks,
  has_terminated: BoxedShaderLoadStore<Node<Bool>>,
}

const TASK_NOT_SPAWNED: u32 = u32::MAX;
const TASK_SPAWNED_FAILED: u32 = u32::MAX - 1;

#[derive(Clone)]
pub struct RayLaunchSizeBuffer {
  pub launch_size: StorageBufferReadonlyDataView<Vec3<u32>>,
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
#[derive(Clone)]
pub struct RayLaunchSizeInvocation {
  launch_size: ShaderReadonlyPtrOf<Vec3<u32>>,
}
impl RayLaunchSizeInvocation {
  pub fn get(&self) -> Node<Vec3<u32>> {
    self.launch_size.load()
  }
}

#[derive(Copy, Clone)]
pub struct RayLaunchRawInfo {
  pub launch_id: Node<Vec3<u32>>,
  pub launch_size: Node<Vec3<u32>>,
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

        let skip_closest = (trace_payload.ray_flags
          & val(RayFlagConfigRaw::RAY_FLAG_SKIP_CLOSEST_HIT_SHADER as u32))
        .greater_than(val(0));

        let ray_sbt_config = RaySBTConfig {
          offset: trace_payload.sbt_ray_config_offset,
          stride: trace_payload.sbt_ray_config_stride,
        };

        let closest_hit = self.tlas_sys.traverse(
          trace_payload,
          self.untyped_payloads.clone(),
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
            let r = val(ANYHIT_BEHAVIOR_ACCEPT_HIT).make_local_var();

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
          if_by(skip_closest.not(), || {
            let hit_group = closest_hit
              .payload
              .hit_ctx
              .compute_sbt_hit_group(ray_sbt_config);

            let closest_payload = ENode::<RayClosestHitCtxPayload> {
              ray_info: trace_payload_all_expand.trace_call,
              hit_ctx: hit_ctx_storage_from_hit_ctx(&closest_hit.payload.hit_ctx),
              hit: hit_storage_from_hit(&closest_hit.payload.hit),
            }
            .construct();

            let closest_shader_index = self.sbt.get_closest_handle(current_sbt, hit_group);
            if_by(closest_shader_index.not_equals(val(u32::MAX)), || {
              let closest_task_index = sbt_task_mapping.get_closest_task(closest_shader_index);
              let sub_task_id = spawn_dynamic(
                &self.info.closest_tasks,
                &self.downstream,
                closest_task_index,
                closest_payload.cast_untyped_node(),
                &RayClosestHitCtxPayload::sized_ty(),
                self.untyped_payloads.clone(),
                trace_payload.payload_ref,
                ctx.generate_self_as_parent(),
              );

              let ty = trace_payload_all.sub_task_ty();
              ty.store(closest_task_index);
              let id = trace_payload_all.sub_task_id();
              id.store(sub_task_id);
            });
          });
        })
        .else_by(|| {
          let missing_payload = ENode::<RayMissHitCtxPayload> {
            ray_info: trace_payload_all_expand.trace_call,
          }
          .construct();

          let miss_sbt_index = self
            .sbt
            .get_missing_handle(current_sbt, trace_payload.miss_index);
          if_by(miss_sbt_index.not_equals(val(u32::MAX)), || {
            let miss_task_index = sbt_task_mapping.get_miss_task(miss_sbt_index);
            let sub_task_id = spawn_dynamic(
              &self.info.missing_tasks,
              &self.downstream,
              miss_task_index,
              missing_payload.cast_untyped_node(),
              &RayMissHitCtxPayload::sized_ty(),
              self.untyped_payloads.clone(),
              trace_payload.payload_ref,
              ctx.generate_self_as_parent(),
            );

            let ty = trace_payload_all.sub_task_ty();
            ty.store(miss_task_index);
            let id = trace_payload_all.sub_task_id();
            id.store(sub_task_id);
          });
        });
      },
    );

    // todo, reduce barrier usage;
    storage_barrier();
    let task_spawn_state = trace_payload_all.sub_task_id().load();
    let task_not_spawn = task_spawn_state
      .equals(TASK_SPAWNED_FAILED)
      .or(task_spawn_state.equals(TASK_NOT_SPAWNED));

    let final_poll_resolved = val(false).make_local_var();

    if_by(task_not_spawn, || {
      // if task not spawned then we direct mark self as terminated
      if_by(
        self.has_terminated.abstract_load().into_bool().not(),
        || {
          // if nothing to spawn, we copy back the user passed in payload, because the payload may be modified in any hit
          let trace_payload = trace_payload_all.trace_call();
          let payload_u32_len = trace_payload.payload_u32_len().load();
          let trace_payload_idx = trace_payload.payload_ref();
          let current_payload_idx = trace_payload_idx.load();

          let (idx, success) =
            self
              .payload_read_back_bumper
              .bump_allocate_by(payload_u32_len, |target, offset| {
                let u32_copy_count = val(0_u32).make_local_var();
                loop_by(|cx| {
                  let data = self
                    .untyped_payloads
                    .index(current_payload_idx + u32_copy_count.load())
                    .load();
                  target.index(offset + u32_copy_count.load()).store(data);
                  u32_copy_count.store(u32_copy_count.load() + val(1));
                  if_by(u32_copy_count.load().equals(payload_u32_len), || {
                    cx.do_break()
                  });
                });
              });

          shader_assert(success);
          trace_payload_idx.store(idx);

          self.has_terminated.abstract_store(val(true.into()));
          final_poll_resolved.store(true);
        },
      );
    });
    storage_barrier();

    let should_poll = self.has_terminated.abstract_load().into_bool().not();
    if_by(should_poll, || {
      let trace_payload_all = ctx.access_self_payload::<TraceTaskSelfPayload>();
      let tid = trace_payload_all.sub_task_ty().load();
      let id = trace_payload_all.sub_task_id().load();

      let trace_payload = trace_payload_all.trace_call();
      let trace_payload_idx = trace_payload.payload_ref();

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

      if_by(missing_task_resolved.or(closest_resolved), || {
        self.has_terminated.abstract_store(val(true.into()));
        final_poll_resolved.store(true);
      });
    });

    (final_poll_resolved, ()).into()
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
  untyped_payload_arr: ShaderPtrOf<[u32]>,
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
      let payload = user_defined_payload_ty_desc.load_from_u32_buffer(
        &untyped_payload_arr,
        untyped_payload_idx,
        StructLayoutTarget::Packed,
      );

      let desc =
        create_composite_task_payload_desc(*id, user_defined_payload_ty_desc, ray_payload_desc);

      let combined = ShaderNodeExpr::Compose {
        target: desc.clone(),
        parameters: vec![ray_payload.handle(), payload],
      }
      .insert_api_raw();

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
    argument_read_back: impl FnOnce(BoxedShaderPtr) + Copy,
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
    payload: ShaderNodeRawHandle,
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
        let (idx, success) = bumper_read_back.bump_allocate_by(
          val(payload_ty_desc.u32_size_count(StructLayoutTarget::Packed)),
          |target, offset| {
            let user_defined_payload = task_payload_node.field_index(1);
            payload_ty_desc.store_into_u32_buffer(
              user_defined_payload.load(),
              &target,
              offset,
              StructLayoutTarget::Packed,
            )
          },
        );
        shader_assert(success);
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
  pub(crate) payload_spawn_bumper: DeviceBumpAllocationInstance<u32>,
  pub(crate) payload_read_back: DeviceBumpAllocationInstance<u32>,
}

impl TracingTaskSpawnerImplSource {
  pub fn create_invocation(
    &self,
    cx: &mut DeviceTaskSystemBuildCtx,
  ) -> TracingTaskSpawnerInvocation {
    TracingTaskSpawnerInvocation {
      payload_spawn_bumper: self
        .payload_spawn_bumper
        .build_allocator_shader(cx.compute_cx),
      payload_read_back: self.payload_read_back.build_allocator_shader(cx.compute_cx),
    }
  }

  pub fn bind(&self, builder: &mut BindingBuilder) {
    self.payload_spawn_bumper.bind_allocator(builder);
    self.payload_read_back.bind_allocator(builder);
  }
}

#[derive(Clone)]
pub(crate) struct TracingTaskSpawnerInvocation {
  pub(crate) payload_spawn_bumper: DeviceBumpAllocationInvocationInstance<u32>,
  pub(crate) payload_read_back: DeviceBumpAllocationInvocationInstance<u32>,
}

impl TracingTaskSpawnerInvocation {
  fn spawn_new_tracing_task(
    &self,
    task_group: &TaskGroupDeviceInvocationInstanceLateResolved,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
    parent: TaskParentRef,
    tcx: &TracingCtx,
  ) -> TracingFutureInvocationRightValue {
    let task_handle = val(u32::MAX).make_local_var();

    if_by(should_trace, || {
      let (launch_id, launch_size) = if let Some(ray_gen) = tcx.ray_gen_ctx() {
        (ray_gen.launch_id(), ray_gen.launch_size())
      } else if let Some(closest) = tcx.closest_hit_ctx() {
        (closest.launch_id(), closest.launch_size())
      } else if let Some(missing) = tcx.miss_hit_ctx() {
        (missing.launch_id(), missing.launch_size())
      } else {
        unreachable!()
      };

      let payload_size = payload_ty.u32_size_count(StructLayoutTarget::Packed);

      let (write_idx, success) =
        self
          .payload_spawn_bumper
          .bump_allocate_by(val(payload_size), |storage, write_idx| {
            payload_ty.store_into_u32_buffer(
              payload,
              &storage,
              write_idx,
              StructLayoutTarget::Packed,
            );
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
        payload_u32_len: val(payload_size),
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

    let inner = TaskFutureInvocationRightValue {
      task_handle: task_handle.load(),
    };
    TracingFutureInvocationRightValue { inner }
  }

  fn read_back_user_payload(
    &mut self,
    payload_ty: ShaderSizedValueType,
    payload_ref: Node<u32>,
  ) -> ShaderNodeRawHandle {
    payload_ty.load_from_u32_buffer(
      &self.payload_read_back.storage,
      payload_ref,
      StructLayoutTarget::Packed,
    )
  }
}

pub const TRACING_TASK_INDEX: usize = 0;

impl<F, T, O, P> ShaderFutureProvider for TraceNextRay<F, T>
where
  T: ShaderFutureProvider<Output = O>,
  F: FnOnce(&O, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
  P: ShaderSizedValueNodeType + Default + Copy,
  O: ShaderAbstractRightValue + Default,
{
  type Output = (O, Node<P>);
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<(O, Node<P>)> {
    let next_trace_logic = self.next_trace_logic;
    self
      .upstream
      .build_device_future(ctx)
      .then(
        move |o, then_invocation, cx| {
          let ctx = cx.invocation_registry.get_mut::<TracingCtx>().unwrap();
          let (should_trace, trace_call, payload) = next_trace_logic(&o, ctx);
          cx.spawn_new_tracing_task(should_trace, trace_call, payload, then_invocation)
        },
        TracingFuture::default(),
      )
      .into_dyn()
  }
}

pub trait DeviceTaskSystemPollCtxTracingExt {
  fn spawn_new_tracing_task<P: ShaderSizedValueNodeType>(
    &self,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: Node<P>,
    state: &TracingFutureInvocation<P>,
  ) -> TracingFutureInvocationRightValue;
}

impl DeviceTaskSystemPollCtxTracingExt for DeviceTaskSystemPollCtx<'_> {
  fn spawn_new_tracing_task<P: ShaderSizedValueNodeType>(
    &self,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: Node<P>,
    state: &TracingFutureInvocation<P>,
  ) -> TracingFutureInvocationRightValue {
    let ctx = self.invocation_registry.get::<TracingCtx>().unwrap();

    let parent = self.generate_self_as_parent();
    let spawner = self
      .invocation_registry
      .get::<TracingTaskSpawnerInvocation>()
      .unwrap();

    spawner.spawn_new_tracing_task(
      &state.inner_task.spawner,
      should_trace,
      trace_call,
      payload.handle(),
      P::sized_ty(),
      parent,
      ctx,
    )
  }
}

pub struct TracingFuture<T> {
  pub inner_task: TaskFuture<TraceTaskSelfPayload>,
  phantom: PhantomData<T>,
}

impl<T> Default for TracingFuture<T> {
  fn default() -> Self {
    Self {
      inner_task: TaskFuture::new(TRACING_TASK_INDEX),
      phantom: PhantomData,
    }
  }
}

impl<T> ShaderFuture for TracingFuture<T>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type Output = Node<T>;

  type Invocation = TracingFutureInvocation<T>;

  fn required_poll_count(&self) -> usize {
    self.inner_task.required_poll_count()
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    TracingFutureInvocation {
      inner_task: self.inner_task.build_poll(ctx),
      phantom: PhantomData,
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.inner_task.bind_input(builder);
  }
}

/// this struct wraps around TaskFutureInvocation, store the state of the flying tracing task
pub struct TracingFutureInvocation<T> {
  pub inner_task: TaskFutureInvocation<TraceTaskSelfPayload>,
  phantom: PhantomData<T>,
}

impl<T> TracingFutureInvocation<T> {
  pub fn task_has_already_resolved(&self) -> Node<bool> {
    self.inner_task.task_has_already_resolved()
  }
  pub fn task_not_allocated(&self) -> Node<bool> {
    self.inner_task.task_not_allocated()
  }
  pub fn task_not_exist(&self) -> Node<bool> {
    self.inner_task.task_not_exist()
  }
}

impl<T> ShaderFutureInvocation for TracingFutureInvocation<T>
where
  T: ShaderSizedValueNodeType + Default + Copy,
{
  type Output = Node<T>;

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let inner = self.inner_task.device_poll(ctx);
    let payload = make_local_var::<T>();
    if_by(inner.is_resolved(), || {
      let user_payload = ctx
        .invocation_registry
        .get_mut::<TracingTaskSpawnerInvocation>()
        .unwrap()
        .read_back_user_payload(
          T::sized_ty(),
          inner.payload.expand().trace_call.expand().payload_ref, // todo reduce unnecessary load
        );

      payload.store(unsafe { user_payload.into_node() });
    });
    (inner.resolved, payload.load()).into()
  }
}

pub struct TracingFutureInvocationRightValue {
  inner: TaskFutureInvocationRightValue,
}

impl<T> ShaderAbstractLeftValue for TracingFutureInvocation<T> {
  type RightValue = TracingFutureInvocationRightValue;

  fn abstract_load(&self) -> Self::RightValue {
    TracingFutureInvocationRightValue {
      inner: self.inner_task.abstract_load(),
    }
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.inner_task.abstract_store(payload.inner);
  }
}
