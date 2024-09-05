use crate::*;

#[derive(Clone, Copy)]
pub struct HitInfo {
  /// gl_HitKindEXT
  pub hit_kind: Node<u32>,
  /// gl_HitTEXT (in world space)
  pub hit_distance: Node<f32>,
}

#[derive(Clone, Copy)]
pub struct HitCtxInfo {
  /// gl_PrimitiveID
  pub primitive_id: Node<u32>,
  /// gl_InstanceID
  ///
  /// index in tlas instance list
  pub instance_id: Node<u32>,
  /// tlas instance sbt offset is not exposed in shader, so we should not exposed in our api.
  pub(crate) instance_sbt_offset: Node<u32>,
  /// gl_InstanceCustomIndexEXT
  ///
  /// provided by user: TopLevelAccelerationStructureSourceInstance.instance_custom_index
  pub instance_custom_id: Node<u32>,
  /// gl_GeometryIndexEXT
  ///
  /// is index in blas geometry list
  pub geometry_id: Node<u32>,
  /// gl_ObjectToWorldEXT
  pub object_to_world: Node<Mat4<f32>>,
  /// gl_WorldToObjectEXT
  pub world_to_object: Node<Mat4<f32>>,
  /// gl_ObjectRayOriginEXT and gl_ObjectRayDirectionEXT
  pub object_space_ray: ShaderRay,
}

impl HitCtxInfo {
  /// The shader record to call is determined by parameters set on the instance, trace ray call, and
  /// the order of geometries in the bottom-level acceleration structure. These parameters are set on
  /// both the host and device during different parts of the scene and pipeline setup and execution
  pub fn compute_sbt_hit_group(&self, ray: RaySBTConfig) -> Node<u32> {
    ray.offset + ray.stride * self.geometry_id + self.instance_sbt_offset
  }
}

#[derive(Clone, Copy)]
pub struct RayLaunchInfo {
  /// gl_LaunchIDEXT
  pub launch_id: Node<Vec3<u32>>,
  /// gl_LaunchSizeEXT
  pub launch_size: Node<Vec3<u32>>,
}

#[derive(Clone, Copy)]
pub struct WorldRayInfo {
  /// gl_WorldRayOriginEXT and gl_WorldRayDirectionEXT
  pub world_ray: ShaderRay,
  /// gl_RayTminEXT and gl_RayTmaxEXT (always in world space)
  pub ray_range: ShaderRayRange,
  /// gl_IncomingRayFlagsEXT
  pub ray_flags: Node<u32>,
}

pub struct RayGenShaderCtx {
  pub launch_info: RayLaunchInfo,
}

#[derive(Clone, Copy)]
pub struct RayClosestHitCtx {
  pub launch_info: RayLaunchInfo,
  pub world_ray: WorldRayInfo,
  pub hit_ctx: HitCtxInfo,
  pub hit: HitInfo,
}

#[derive(Clone, Copy)]
pub struct RayMissCtx {
  pub launch_info: RayLaunchInfo,
  pub world_ray: WorldRayInfo,
}

pub struct RayAnyHitCtx {
  pub launch_info: RayLaunchInfo,
  pub world_ray: WorldRayInfo,
  pub hit_ctx: HitCtxInfo,
  pub hit: HitInfo,
}

pub struct RayIntersectCtx {
  pub launch_info: RayLaunchInfo,
  pub world_ray: WorldRayInfo,
  pub hit_ctx: HitCtxInfo,
}

pub struct TracingCtx {
  pub(crate) missing: Option<Box<dyn MissingHitCtxProvider>>,
  pub(crate) closest: Option<Box<dyn ClosestHitCtxProvider>>,
  pub(crate) payload: Option<(StorageNode<AnyType>, ShaderSizedValueType)>,
}

impl TracingCtx {
  pub fn miss_hit_ctx(&self) -> Option<&dyn MissingHitCtxProvider> {
    self.missing.as_deref()
  }
  pub fn closest_hit_ctx(&self) -> Option<&dyn ClosestHitCtxProvider> {
    self.closest.as_deref()
  }

  pub fn payload<T: ShaderSizedValueNodeType>(&self) -> StorageNode<T> {
    let payload = self.payload.as_ref().unwrap();
    assert_eq!(&T::sized_ty(), &payload.1);
    unsafe { payload.0.cast_type() }
  }
}

pub trait WorldRayInfoProvider {
  /// gl_WorldRayOriginEXT and gl_WorldRayDirectionEXT
  fn world_ray(&self) -> ShaderRay;
  /// gl_RayTminEXT and gl_RayTmaxEXT (always in world space)
  fn ray_range(&self) -> ShaderRayRange;
  /// gl_IncomingRayFlagsEXT
  fn ray_flags(&self) -> Node<u32>;
}
pub trait RayLaunchInfoProvider {
  /// gl_LaunchIDEXT
  fn launch_id(&self) -> Node<Vec3<u32>>;
  /// gl_LaunchSizeEXT
  fn launch_size(&self) -> Node<Vec3<u32>>;
}

pub trait MissingHitCtxProvider: WorldRayInfoProvider + RayLaunchInfoProvider {}
pub trait ClosestHitCtxProvider: WorldRayInfoProvider + RayLaunchInfoProvider {
  /// gl_PrimitiveID
  fn primitive_id(&self) -> Node<u32>;
  /// gl_InstanceID
  ///
  /// index in tlas instance list
  fn instance_id(&self) -> Node<u32>;
  /// gl_InstanceCustomIndexEXT
  ///
  /// provided by user: TopLevelAccelerationStructureSourceInstance.instance_custom_index
  fn instance_custom_id(&self) -> Node<u32>;
  /// gl_GeometryIndexEXT
  ///
  /// is index in blas geometry list
  fn geometry_id(&self) -> Node<u32>;
  /// gl_ObjectToWorldEXT
  fn object_to_world(&self) -> Node<Mat4<f32>>;
  /// gl_WorldToObjectEXT
  fn world_to_object(&self) -> Node<Mat4<f32>>;
  /// gl_ObjectRayOriginEXT and gl_ObjectRayDirectionEXT
  fn object_space_ray(&self) -> ShaderRay;

  /// gl_HitKindEXT
  fn hit_kind(&self) -> Node<u32>;
  /// gl_HitTEXT (in world space)
  fn hit_distance(&self) -> Node<f32>;
}
