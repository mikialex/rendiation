use crate::*;

pub enum RayTracingShaderStage {
  RayGeneration,
  ClosestHit,
  AnyHit,
  Miss,
  Intersection,
}

pub struct ShaderTLAS;

pub struct TLAS;
pub struct BLAS;

pub struct ShaderRay {
  pub origin: Node<Vec3<f32>>,
  pub direction: Node<Vec3<f32>>,
}

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

pub struct ShaderRecord;

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

pub struct MeshSBTConfig {
  /// the index of self in building TLAS
  pub tlas_idx: Node<u32>,
  /// starting offset within the SBT where its sub-table of hit group records start.
  pub sbt_offset: Node<u32>,
}

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

impl RayFlagConfig {
  pub fn into_raw_ray_flags(self) -> u32 {
    todo!()
  }
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

/// mainly used in missing stage
trait RayBaseShaderStageCtx {
  fn world_ray(&self) -> ShaderRay;
  // in world semantic
  fn ray_range(&self) -> ShaderRayRange;

  fn ray_flags(&self) -> Node<u32>;
}

trait RayIntersectionShaderStageCtx: RayBaseShaderStageCtx {
  fn local_ray(&self) -> ShaderRay;

  fn world_hit_info(&self) -> WorldHitInfo;
  fn local_world_transform(&self) -> MeshWorldObjectTransform;
}

/// used as closest or any hit
trait RayHitShaderStageCtx: RayIntersectionShaderStageCtx {
  fn ray_hit_distance(&self) -> Node<f32>;
  /// https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_tracing.txt#L796
  fn hit_kind(&self) -> Node<u32>;
}
