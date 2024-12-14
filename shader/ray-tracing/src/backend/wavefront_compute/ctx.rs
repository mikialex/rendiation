use std::hash::Hash;

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
  pub launch_size: Vec3<u32>,
  pub launch_id: Vec3<u32>,
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
pub struct HitStorage {
  /// gl_HitKindEXT
  pub hit_kind: u32,
  /// gl_HitTEXT (in world space)
  pub hit_distance: f32,
  /// attribute for anyhit and closest shader, is bary_coord for triangle geometry
  /// todo support with custom attribute for intersection shader
  pub hit_attribute: HitAttribute,
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

pub fn hit_storage_from_hit(hit: &HitInfo) -> Node<HitStorage> {
  ENode::<HitStorage> {
    hit_kind: hit.hit_kind,
    hit_distance: hit.hit_distance,
    hit_attribute: hit.hit_attribute,
  }
  .construct()
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
  pub hit: HitStorage,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, StorageNodePtrAccess)]
pub struct RayMissHitCtxPayload {
  pub ray_info: ShaderRayTraceCallStoragePayload,
}

pub struct WaveFrontTracingBaseProvider;

impl TraceFutureBaseProvider for WaveFrontTracingBaseProvider {
  fn create_ray_gen_shader_base(&self) -> Box<dyn TraceOperator<()>> {
    Box::new(TracingCtxProviderTracer {
      stage: RayTraceableShaderStage::RayGeneration,
      payload_ty: None,
    })
  }

  fn create_closest_hit_shader_base(&self, ty: ShaderSizedValueType) -> Box<dyn TraceOperator<()>> {
    Box::new(TracingCtxProviderTracer {
      stage: RayTraceableShaderStage::ClosestHit,
      payload_ty: Some(ty),
    })
  }

  fn create_miss_hit_shader_base(&self, ty: ShaderSizedValueType) -> Box<dyn TraceOperator<()>> {
    Box::new(TracingCtxProviderTracer {
      stage: RayTraceableShaderStage::Miss,
      payload_ty: Some(ty),
    })
  }
}

#[derive(Clone)]
struct TracingCtxProviderTracer {
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
}

impl ShaderHashProvider for TracingCtxProviderTracer {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.stage.hash(hasher);
    self.payload_ty.hash(hasher);
  }
  shader_hash_type_id! {}
}

impl ShaderFutureProvider for TracingCtxProviderTracer {
  type Output = ();
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<()> {
    TracingCtxProviderFuture {
      stage: self.stage,
      payload_ty: self.payload_ty.clone(),
      ray_spawner: ctx
        .get_mut::<TracingTaskSpawnerImplSource>()
        .unwrap()
        .clone(),
      launch_size: ctx.get_mut::<RayLaunchSizeBuffer>().unwrap().clone(),
      base: Default::default(),
    }
    .into_dyn()
  }
}
impl NativeRayTracingShaderBuilder for TracingCtxProviderTracer {
  type Output = ();
  fn build(&self, _: &mut dyn NativeRayTracingShaderCtx) {}
  fn bind(&self, _: &mut BindingBuilder) {}
}

pub struct TracingCtxProviderFuture {
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
  ray_spawner: TracingTaskSpawnerImplSource,
  launch_size: RayLaunchSizeBuffer,
  base: BaseShaderFuture<()>,
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
      launch_size: self.launch_size.build(cx),
      base: self.base.build_poll(cx),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.ray_spawner.bind(builder);
    self.launch_size.bind(builder);
  }
}

pub struct TracingCtxProviderFutureInvocation {
  base: BaseFutureInvocation<()>,
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
  ray_spawner: TracingTaskSpawnerInvocation,
  launch_size: RayLaunchSizeInvocation,
}
impl ShaderFutureInvocation for TracingCtxProviderFutureInvocation {
  type Output = ();
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<()> {
    let r = self.base.device_poll(ctx);
    // store pointers in closures so that payload/ctx will be reloaded when shader is awakened

    // accessing task{}_payload_with_ray, see fn spawn_dynamic in trace_task.rs
    let combined_payload = ctx.access_self_payload_untyped();
    let user_defined_payload = if matches!(self.stage, RayTraceableShaderStage::RayGeneration) {
      None
    } else {
      let payload_ty = self.payload_ty.clone().unwrap();
      // todo fix fall back task access
      let user_defined_payload: StorageNode<AnyType> =
        unsafe { index_access_field(combined_payload.handle(), 1) };
      Some((user_defined_payload, payload_ty.clone()))
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

    let launch_size = self.launch_size;
    let ray_gen = matches!(self.stage, RayTraceableShaderStage::RayGeneration).then(|| {
      // ray_gen payload is global id. see trace_ray.
      let ray_gen_payload_ptr: StorageNode<Vec3<u32>> = ctx.access_self_payload();
      let launch_id = ray_gen_payload_ptr.load();
      let info = RayLaunchRawInfo {
        launch_id,
        launch_size: launch_size.get(),
      };
      Box::new(info) as Box<dyn RayGenCtxProvider>
    });

    ctx.invocation_registry.register(TracingCtx {
      ray_gen,
      missing,
      closest,
      payload: user_defined_payload,
      registry: Default::default(),
    });
    ctx.invocation_registry.register(self.ray_spawner.clone());

    (r.resolved, ()).into()
  }
}

impl RayLaunchInfoProvider for StorageNode<RayClosestHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    let node = RayClosestHitCtxPayload::storage_node_ray_info_field_ptr(*self);
    ShaderRayTraceCallStoragePayload::storage_node_launch_id_field_ptr(node).load()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    let node = RayClosestHitCtxPayload::storage_node_ray_info_field_ptr(*self);
    ShaderRayTraceCallStoragePayload::storage_node_launch_size_field_ptr(node).load()
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
    let ctx = RayClosestHitCtxPayload::storage_node_hit_field_ptr(*self);
    HitStorage::storage_node_hit_kind_field_ptr(ctx).load()
  }

  fn hit_distance(&self) -> Node<f32> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_field_ptr(*self);
    HitStorage::storage_node_hit_distance_field_ptr(ctx).load()
  }

  fn hit_attribute(&self) -> Node<HitAttribute> {
    let ctx = RayClosestHitCtxPayload::storage_node_hit_field_ptr(*self);
    HitStorage::storage_node_hit_attribute_field_ptr(ctx).load()
  }
}

impl RayLaunchInfoProvider for StorageNode<RayMissHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    let node = RayMissHitCtxPayload::storage_node_ray_info_field_ptr(*self);
    ShaderRayTraceCallStoragePayload::storage_node_launch_id_field_ptr(node).load()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    let node = RayMissHitCtxPayload::storage_node_ray_info_field_ptr(*self);
    ShaderRayTraceCallStoragePayload::storage_node_launch_size_field_ptr(node).load()
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
