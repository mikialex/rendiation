use std::hash::Hash;

use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, Default)]
pub struct TraceTaskSelfPayload {
  pub sub_task_ty: u32,
  pub sub_task_id: u32,
  pub trace_call: ShaderRayTraceCallStoragePayload,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy, Default)]
pub struct ShaderRayTraceCallStoragePayload {
  pub launch_size: Vec3<u32>,
  pub launch_id: Vec3<u32>,
  pub payload_ref: u32,
  pub payload_u32_len: u32,
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
#[derive(ShaderStruct, Clone, Copy)]
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
#[derive(ShaderStruct, Clone, Copy)]
pub struct HitCtxStorage {
  pub primitive_id: u32,
  pub instance_id: u32,
  pub instance_sbt_offset: u32,
  pub instance_custom_id: u32,
  pub geometry_id: u32,
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
    object_space_ray_origin: hit_ctx.object_space_ray.origin,
    object_space_ray_direction: hit_ctx.object_space_ray.direction,
  }
  .construct()
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct RayClosestHitCtxPayload {
  pub ray_info: ShaderRayTraceCallStoragePayload,
  pub hit_ctx: HitCtxStorage,
  pub hit: HitStorage,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
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
      tlas_sys: ctx
        .get::<Box<dyn GPUAccelerationStructureSystemTlasCompImplInstance>>()
        .unwrap()
        .clone(),
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
  // only effective in closest stage
  tlas_sys: Box<dyn GPUAccelerationStructureSystemTlasCompImplInstance>,
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
      tlas_sys: matches!(self.stage, RayTraceableShaderStage::ClosestHit)
        .then(|| self.tlas_sys.build_shader(cx.compute_cx)),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.ray_spawner.bind(builder);
    self.launch_size.bind(builder);
    matches!(self.stage, RayTraceableShaderStage::ClosestHit)
      .then(|| self.tlas_sys.bind_pass(builder.binder));
  }
}

pub struct TracingCtxProviderFutureInvocation {
  base: BaseFutureInvocation<()>,
  stage: RayTraceableShaderStage,
  payload_ty: Option<ShaderSizedValueType>,
  ray_spawner: TracingTaskSpawnerInvocation,
  launch_size: RayLaunchSizeInvocation,
  // Some in closest stage, None in other stages
  tlas_sys: Option<Box<dyn GPUAccelerationStructureSystemTlasCompImplInvocation>>,
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
      let user_defined_payload = combined_payload.field_index(1);
      Some((user_defined_payload, payload_ty.clone()))
    };

    let missing = matches!(self.stage, RayTraceableShaderStage::Miss).then(|| {
      let ray_payload = combined_payload.field_index(1);
      let ray_payload = RayMissHitCtxPayload::create_accessor_from_raw_ptr(ray_payload);
      Box::new(ray_payload) as Box<dyn MissingHitCtxProvider>
    });

    let closest = matches!(self.stage, RayTraceableShaderStage::ClosestHit).then(|| {
      let ray_payload = combined_payload.field_index(0);
      let ray_payload = RayClosestHitCtxPayload::create_accessor_from_raw_ptr(ray_payload);

      let ctx = ray_payload.hit_ctx();
      let instance_id = ctx.instance_id().load();
      let tlas_sys = self.tlas_sys.as_ref().unwrap();
      let tlas_ptr = tlas_sys.index_tlas(instance_id);
      let ctx = ClosestHitCtx {
        ctx: ray_payload,
        tlas_ptr,
      };
      Box::new(ctx) as Box<dyn ClosestHitCtxProvider>
    });

    let launch_size = self.launch_size.clone();
    let ray_gen = matches!(self.stage, RayTraceableShaderStage::RayGeneration).then(|| {
      // ray_gen payload is global id. see trace_ray.
      let ray_gen_payload_ptr = ctx.access_self_payload::<Vec3<u32>>();
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

impl RayLaunchInfoProvider for ClosestHitCtx {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    self.ctx.ray_info().launch_id().load()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    self.ctx.ray_info().launch_size().load()
  }
}

impl WorldRayInfoProvider for ShaderAccessorOf<ShaderRayTraceCallStoragePayload> {
  fn world_ray(&self) -> ShaderRay {
    let origin = self.ray_origin().load();
    let direction = self.ray_direction().load();
    ShaderRay { origin, direction }
  }

  fn ray_range(&self) -> ShaderRayRange {
    let range = self.range().load();
    ShaderRayRange {
      min: range.x(),
      max: range.y(),
    }
  }

  fn ray_flags(&self) -> Node<u32> {
    self.ray_flags().load()
  }
}

#[derive(Clone)]
struct ClosestHitCtx {
  ctx: ShaderAccessorOf<RayClosestHitCtxPayload>,
  tlas_ptr: ShaderReadonlyAccessorOf<TopLevelAccelerationStructureSourceDeviceInstance>,
}

impl WorldRayInfoProvider for ClosestHitCtx {
  fn world_ray(&self) -> ShaderRay {
    self.ctx.ray_info().world_ray()
  }

  fn ray_range(&self) -> ShaderRayRange {
    self.ctx.ray_info().ray_range()
  }

  fn ray_flags(&self) -> Node<u32> {
    self.ctx.ray_info().ray_flags().load()
  }
}

impl ClosestHitCtxProvider for ClosestHitCtx {
  fn primitive_id(&self) -> Node<u32> {
    self.ctx.hit_ctx().primitive_id().load()
  }

  fn instance_id(&self) -> Node<u32> {
    self.ctx.hit_ctx().instance_id().load()
  }

  fn instance_custom_id(&self) -> Node<u32> {
    self.ctx.hit_ctx().instance_custom_id().load()
  }

  fn geometry_id(&self) -> Node<u32> {
    self.ctx.hit_ctx().geometry_id().load()
  }

  fn object_to_world(&self) -> Node<Mat4<f32>> {
    self.tlas_ptr.transform().load()
  }

  fn world_to_object(&self) -> Node<Mat4<f32>> {
    self.tlas_ptr.transform_inv().load()
  }

  fn object_space_ray(&self) -> ShaderRay {
    let ctx = self.ctx.hit_ctx();
    let direction = ctx.object_space_ray_direction().load();
    let origin = ctx.object_space_ray_origin().load();
    ShaderRay { origin, direction }
  }

  fn hit_kind(&self) -> Node<u32> {
    self.ctx.hit().hit_kind().load()
  }

  fn hit_distance(&self) -> Node<f32> {
    self.ctx.hit().hit_distance().load()
  }

  fn hit_attribute(&self) -> Node<HitAttribute> {
    self.ctx.hit().hit_attribute().load()
  }
}

impl RayLaunchInfoProvider for ShaderAccessorOf<RayMissHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    self.ray_info().launch_id().load()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    self.ray_info().launch_size().load()
  }
}
impl WorldRayInfoProvider for ShaderAccessorOf<RayMissHitCtxPayload> {
  fn world_ray(&self) -> ShaderRay {
    self.ray_info().world_ray()
  }

  fn ray_range(&self) -> ShaderRayRange {
    self.ray_info().ray_range()
  }

  fn ray_flags(&self) -> Node<u32> {
    self.ray_info().ray_flags().load()
  }
}
impl MissingHitCtxProvider for ShaderAccessorOf<RayMissHitCtxPayload> {}
