use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, StorageNodePtrAccess, Default)]
pub struct TraceTaskSelfPayload {
  pub sub_task_ty: u32,
  pub sub_task_id: u32,
  pub trace_call: ShaderRayTraceCallStoragePayload,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, StorageNodePtrAccess, Default)]
pub struct ShaderRayTraceCallStoragePayload {
  pub payload_ref: u32,
  pub tlas_idx: u32,
  pub ray_flags: u32,
  pub cull_mask: u32,
  pub sbt_ray_config_offset: u32,
  pub sbt_ray_config_stride: u32,
  pub miss_index: u32,
  pub ray_origin: Vec3<f32>,
  pub ray_direction: Vec3<f32>,
  pub range: Vec2<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, StorageNodePtrAccess)]
pub struct HitCtxStorage {
  pub primitive_id: u32,
  pub instance_id: u32,
  pub instance_sbt_offset: u32,
  pub instance_custom_id: u32,
  pub geometry_id: u32,
  pub object_to_world: Mat4<f32>,
  pub world_to_object: Mat4<f32>,
  pub object_space_ray_origin: Vec3<f32>,
  pub object_space_ray_direction: Vec3<f32>,
}

pub fn hit_ctx_storage_from_hit_ctx(hit_ctx: &HitCtxInfo) -> Node<HitCtxStorage> {
  ENode::<HitCtxStorage> {
    primitive_id: hit_ctx.primitive_id,
    instance_id: hit_ctx.instance_id,
    instance_sbt_offset: hit_ctx.instance_sbt_offset,
    instance_custom_id: hit_ctx.instance_custom_id,
    geometry_id: hit_ctx.geometry_id,
    object_to_world: hit_ctx.object_to_world,
    world_to_object: hit_ctx.world_to_object,
    object_space_ray_origin: hit_ctx.object_space_ray.origin,
    object_space_ray_direction: hit_ctx.object_space_ray.direction,
  }
  .construct()
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, StorageNodePtrAccess)]
pub struct RayClosestHitCtxPayload {
  pub ray_info: ShaderRayTraceCallStoragePayload,
  pub hit_ctx: HitCtxStorage,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, StorageNodePtrAccess)]
pub struct RayMissHitCtxPayload {
  pub ray_info: ShaderRayTraceCallStoragePayload,
}

pub struct WaveFrontTracingBaseProvider;

impl TraceFutureBaseProvider for WaveFrontTracingBaseProvider {
  fn create_ray_gen_shader_base() -> impl TraceOperator<()> {
    TracingCtxProviderTracer {
      stage: RayTraceableShaderStage::RayGeneration,
      payload_ty: None,
    }
  }

  fn create_closest_hit_shader_base<P: ShaderSizedValueNodeType>() -> impl TraceOperator<()> {
    TracingCtxProviderTracer {
      stage: RayTraceableShaderStage::ClosestHit,
      payload_ty: Some(P::sized_ty()),
    }
  }

  fn create_miss_hit_shader_base<P: ShaderSizedValueNodeType>() -> impl TraceOperator<()> {
    TracingCtxProviderTracer {
      stage: RayTraceableShaderStage::Miss,
      payload_ty: Some(P::sized_ty()),
    }
  }
}

struct TracingCtxProviderTracer {
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
}

impl ShaderFutureProvider<()> for TracingCtxProviderTracer {
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<()> {
    TracingCtxProviderFuture {
      stage: self.stage,
      payload_ty: self.payload_ty.clone(),
      ray_spawner: ctx
        .get_mut::<TracingTaskSpawnerImplSource>()
        .unwrap()
        .clone(),
    }
    .into_dyn()
  }
}
impl<T> NativeRayTracingShaderBuilder<T> for TracingCtxProviderTracer
where
  T: Default,
{
  fn build(&self, _: &mut dyn NativeRayTracingShaderCtx) -> T {
    // todo, register ctx
    T::default()
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}

pub struct TracingCtxProviderFuture {
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
  ray_spawner: TracingTaskSpawnerImplSource,
}

impl ShaderFuture for TracingCtxProviderFuture {
  type Output = ();

  type Invocation = TracingCtxProviderFutureInvocation;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(&self, cx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    TracingCtxProviderFutureInvocation {
      stage: self.stage,
      payload_ty: self.payload_ty.clone(),
      ray_spawner: self.ray_spawner.create_invocation(cx),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.ray_spawner.bind(builder)
  }

  fn reset(&mut self, _: &mut DeviceParallelComputeCtx, _: u32) {
    // ray_spawner should be reset by trace task, but not ours
  }
}

pub struct TracingCtxProviderFutureInvocation {
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
  ray_spawner: TracingTaskSpawnerInvocation,
}
impl ShaderFutureInvocation for TracingCtxProviderFutureInvocation {
  type Output = ();
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<()> {
    // accessing task{}_payload_with_ray, see fn spawn_dynamic in trace_task.rs
    let combined_payload = ctx.access_self_payload_untyped();
    let user_defined_payload = if matches!(self.stage, RayTraceableShaderStage::RayGeneration) {
      None
    } else {
      let user_defined_payload: StorageNode<AnyType> =
        unsafe { index_access_field(combined_payload.handle(), 1) };
      Some((user_defined_payload, self.payload_ty.clone().unwrap()))
    };

    let missing = matches!(self.stage, RayTraceableShaderStage::Miss).then(|| unsafe {
      let ray_payload: StorageNode<RayMissHitCtxPayload> =
        index_access_field(combined_payload.handle(), 0);
      Box::new(ray_payload) as Box<dyn MissingHitCtxProvider>
    });

    let closest = matches!(self.stage, RayTraceableShaderStage::ClosestHit).then(|| unsafe {
      let ray_payload: StorageNode<RayClosestHitCtxPayload> =
        index_access_field(combined_payload.handle(), 0);
      Box::new(ray_payload) as Box<dyn ClosestHitCtxProvider>
    });

    ctx.invocation_registry.register(TracingCtx {
      missing,
      closest,
      payload: user_defined_payload,
      registry: Default::default(),
    });

    ctx.invocation_registry.register(self.ray_spawner.clone());

    (val(true), ()).into()
  }
}

impl RayLaunchInfoProvider for StorageNode<RayClosestHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    todo!()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    todo!()
  }
}

impl WorldRayInfoProvider for StorageNode<ShaderRayTraceCallStoragePayload> {
  fn world_ray(&self) -> ShaderRay {
    let origin = ShaderRayTraceCallStoragePayload::storage_node_ray_origin_field_ptr(*self).load();
    let direction =
      ShaderRayTraceCallStoragePayload::storage_node_ray_direction_field_ptr(*self).load();
    ShaderRay { origin, direction }
  }

  fn ray_range(&self) -> ShaderRayRange {
    let range = ShaderRayTraceCallStoragePayload::storage_node_range_field_ptr(*self).load();
    ShaderRayRange {
      min: range.x(),
      max: range.y(),
    }
  }

  fn ray_flags(&self) -> Node<u32> {
    ShaderRayTraceCallStoragePayload::storage_node_ray_flags_field_ptr(*self).load()
  }
}

impl WorldRayInfoProvider for StorageNode<RayClosestHitCtxPayload> {
  fn world_ray(&self) -> ShaderRay {
    RayClosestHitCtxPayload::storage_node_ray_info_field_ptr(*self).world_ray()
  }

  fn ray_range(&self) -> ShaderRayRange {
    RayClosestHitCtxPayload::storage_node_ray_info_field_ptr(*self).ray_range()
  }

  fn ray_flags(&self) -> Node<u32> {
    RayClosestHitCtxPayload::storage_node_ray_info_field_ptr(*self).ray_flags()
  }
}

impl ClosestHitCtxProvider for StorageNode<RayClosestHitCtxPayload> {
  fn primitive_id(&self) -> Node<u32> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    HitCtxStorage::storage_node_primitive_id_field_ptr(ctx).load()
  }

  fn instance_id(&self) -> Node<u32> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    HitCtxStorage::storage_node_instance_id_field_ptr(ctx).load()
  }

  fn instance_custom_id(&self) -> Node<u32> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    HitCtxStorage::storage_node_instance_custom_id_field_ptr(ctx).load()
  }

  fn geometry_id(&self) -> Node<u32> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    HitCtxStorage::storage_node_geometry_id_field_ptr(ctx).load()
  }

  fn object_to_world(&self) -> Node<Mat4<f32>> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    HitCtxStorage::storage_node_object_to_world_field_ptr(ctx).load()
  }

  fn world_to_object(&self) -> Node<Mat4<f32>> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    HitCtxStorage::storage_node_world_to_object_field_ptr(ctx).load()
  }

  fn object_space_ray(&self) -> ShaderRay {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_ctx_field_ptr(*self);
    let direction = HitCtxStorage::storage_node_object_space_ray_direction_field_ptr(ctx).load();
    let origin = HitCtxStorage::storage_node_object_space_ray_origin_field_ptr(ctx).load();
    ShaderRay { origin, direction }
  }

  fn hit_kind(&self) -> Node<u32> {
    todo!()
  }

  fn hit_distance(&self) -> Node<f32> {
    todo!()
  }
}

impl RayLaunchInfoProvider for StorageNode<RayMissHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    todo!()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    todo!()
  }
}
impl WorldRayInfoProvider for StorageNode<RayMissHitCtxPayload> {
  fn world_ray(&self) -> ShaderRay {
    RayMissHitCtxPayload::storage_node_ray_info_field_ptr(*self).world_ray()
  }

  fn ray_range(&self) -> ShaderRayRange {
    RayMissHitCtxPayload::storage_node_ray_info_field_ptr(*self).ray_range()
  }

  fn ray_flags(&self) -> Node<u32> {
    RayMissHitCtxPayload::storage_node_ray_info_field_ptr(*self).ray_flags()
  }
}
impl MissingHitCtxProvider for StorageNode<RayMissHitCtxPayload> {}
