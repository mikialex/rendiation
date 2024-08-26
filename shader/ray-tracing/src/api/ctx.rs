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

impl Default for RayClosestHitCtx {
  fn default() -> Self {
    todo!()
  }
}

#[derive(Clone, Copy)]
pub struct RayClosestHitCtxStore {}

impl ShaderAbstractLeftValue for RayClosestHitCtxStore {
  type RightValue = RayClosestHitCtx;

  fn abstract_load(&self) -> Self::RightValue {
    todo!()
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    todo!()
  }
}

impl ShaderAbstractRightValue for RayClosestHitCtx {
  type AbstractLeftValue = RayClosestHitCtxStore;

  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue {
    todo!()
  }
}

#[derive(Clone, Copy)]
pub struct RayMissCtx {
  pub launch_info: RayLaunchInfo,
  pub world_ray: WorldRayInfo,
}

impl Default for RayMissCtx {
  fn default() -> Self {
    todo!()
  }
}

#[derive(Clone, Copy)]
pub struct RayMissCtxStore {}

impl ShaderAbstractLeftValue for RayMissCtxStore {
  type RightValue = RayMissCtx;

  fn abstract_load(&self) -> Self::RightValue {
    todo!()
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    todo!()
  }
}

impl ShaderAbstractRightValue for RayMissCtx {
  type AbstractLeftValue = RayMissCtxStore;

  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue {
    todo!()
  }
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
