use crate::*;

pub enum RayTracingShaderStage {
  RayGeneration,
  ClosestHit,
  AnyHit,
  Miss,
  Intersection,
}

/// placeholder for future impl
pub struct ShaderTLAS;

#[derive(Clone, Copy)]
pub struct ShaderRay {
  pub origin: Node<Vec3<f32>>,
  pub direction: Node<Vec3<f32>>,
}

#[derive(Clone, Copy)]
pub struct ShaderRayRange {
  /// minimal distance for a ray hit
  ///
  /// must be non-negative
  ///
  /// must be less than or equal to ray_max
  pub min: Node<f32>,

  /// maximum distance for a ray hit
  ///
  /// must be non-negative,
  pub max: Node<f32>,
}

#[derive(Clone, Copy)]
pub struct ShaderRayTraceCall {
  pub tlas: Node<ShaderTLAS>,

  /// https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_tracing.txt#L908
  pub ray_flags: Node<u32>,

  /// <cullMask> is a mask which specifies the instances to be intersected
  /// i.e visible to the traced ray. Only the 8 least-significant bits are used;
  /// other bits are ignored. This mask will be combined with the mask field
  /// specified in VkAccelerationStructureInstanceKHR as defined in the Ray
  /// Traversal chapter of Vulkan Specification using a bitwise AND operation.
  ///  The instance is visible only if the result of the operation is non-zero.
  /// The upper 24 bits of this value are ignored. If the value is zero, no
  /// instances are visible.
  pub cull_mask: Node<u32>,

  pub sbt_hit_group_config: RaySBTConfig,
  pub miss_index: Node<u32>,

  pub ray: ShaderRay,
  pub range: ShaderRayRange,

  pub payload: Node<i32>,
}

pub struct ShaderRayTraceCallLocalVar {}

impl ShaderAbstractLeftValue for ShaderRayTraceCallLocalVar {
  type RightValue = ShaderRayTraceCall;

  fn abstract_load(&self) -> Self::RightValue {
    todo!()
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    todo!()
  }
}

impl ShaderAbstractRightValue for ShaderRayTraceCall {
  type LocalLeftValue = ShaderRayTraceCallLocalVar;

  fn into_local_left_value(self) -> Self::LocalLeftValue {
    todo!()
  }
}

impl Default for ShaderRayTraceCall {
  fn default() -> Self {
    todo!()
  }
}

pub struct ShaderRecord {
  shader: u32,
}

pub struct HitGroupShaderRecord {
  closet_hit: u32,
  any_hit: Option<u32>,
  intersection: Option<u32>,
}

pub struct ShaderBindingTable {
  pub ray_generation: Vec<ShaderRecord>,
  pub ray_miss: Vec<ShaderRecord>,
  pub ray_hit: Vec<ShaderRecord>,
  pub callable: Vec<ShaderRecord>,
}

/// The shader record to call is determined by parameters set on the instance, trace ray call, and
/// the order of geometries in the bottom-level acceleration structure. These parameters are set on
/// both the host and device during different parts of the scene and pipeline setup and execution
pub fn compute_sbt_hit_group(mesh: MeshSBTConfig, ray: RaySBTConfig) -> Node<u32> {
  ray.offset + ray.stride * mesh.tlas_idx + mesh.sbt_offset
}

#[derive(Clone, Copy)]
pub struct MeshSBTConfig {
  /// the index of self in building TLAS
  pub tlas_idx: Node<u32>,
  /// starting offset within the SBT where its sub-table of hit group records start.
  pub sbt_offset: Node<u32>,
}

#[derive(Clone, Copy)]
pub struct RaySBTConfig {
  /// When tracing a ray on the device we can specify an additional SBT offset for the ray, often
  /// referred to as the ray “type”,
  pub offset: Node<u32>,
  /// and an SBT stride, typically referred to as the number of ray “types”
  pub stride: Node<u32>,
}

// https://microsoft.github.io/DirectX-Specs/d3d/Raytracing.html#ray-flags
pub struct RayFlagConfig {
  pub opaque: Option<RayFlagOpaqueBehavior>,
  pub primitive: Option<RayFlagPrimitiveBehavior>,
  pub accept_first_hit_and_end_search: bool,
  pub skip_closet_hit_invocation: bool,
}

#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum RayFlagConfigRaw {
  RAY_FLAG_NONE = 0x00,
  RAY_FLAG_FORCE_OPAQUE = 0x01,
  RAY_FLAG_FORCE_NON_OPAQUE = 0x02,
  RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH = 0x04,
  RAY_FLAG_SKIP_CLOSEST_HIT_SHADER = 0x08,
  RAY_FLAG_CULL_BACK_FACING_TRIANGLES = 0x10,
  RAY_FLAG_CULL_FRONT_FACING_TRIANGLES = 0x20,
  RAY_FLAG_CULL_OPAQUE = 0x40,
  RAY_FLAG_CULL_NON_OPAQUE = 0x80,
  RAY_FLAG_SKIP_TRIANGLES = 0x100,
  RAY_FLAG_SKIP_PROCEDURAL_PRIMITIVES = 0x200,
}

pub enum RayFlagOpaqueBehavior {
  ForceOpaque,
  ForceTransparent,
  CullOpaque,
  CullTransparent,
}

pub enum RayFlagPrimitiveBehavior {
  Normal(RayFlagTriangleCullBehavior),
  SkipAllProceduralPrimitive,
  SkipAllTriangle,
}

pub enum RayFlagTriangleCullBehavior {
  CullFront,
  CullBack,
}

pub struct WorldHitInfo {
  pub primitive_id: Node<u32>,
  pub instance_id: Node<u32>,
}

pub struct MeshWorldObjectTransform {
  pub object_to_world: Node<Mat4<f32>>,
  pub world_to_object: Node<Mat4<f32>>,
}

pub trait RayDispatchShaderStageCtx {
  fn launch_id(&self) -> Node<Vec3<u32>>;
  fn launch_size(&self) -> Node<Vec3<u32>>;
}

/// mainly used in missing stage
pub trait RayBaseShaderStageCtx: RayDispatchShaderStageCtx {
  fn world_ray(&self) -> ShaderRay;
  // in world semantic
  fn ray_range(&self) -> ShaderRayRange;

  fn ray_flags(&self) -> Node<u32>;
}

pub trait RayIntersectionShaderStageCtx: RayBaseShaderStageCtx {
  fn local_ray(&self) -> ShaderRay;

  fn world_hit_info(&self) -> WorldHitInfo;
  fn local_world_transform(&self) -> MeshWorldObjectTransform;
}

/// used as closest or any hit
pub trait RayHitShaderStageCtx: RayIntersectionShaderStageCtx {
  fn ray_hit_distance(&self) -> Node<f32>;
  /// https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_tracing.txt#L796
  fn hit_kind(&self) -> Node<u32>;
}
