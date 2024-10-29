use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingShaderStage {
  RayGeneration,
  ClosestHit,
  AnyHit,
  Miss,
  Intersection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTraceableShaderStage {
  RayGeneration,
  Miss,
  ClosestHit,
}

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
  // todo, use Vec2<u32>, see https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_tracing.txt#L567
  pub tlas_idx: Node<u32>,

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

  pub sbt_ray_config: RaySBTConfig,
  pub miss_index: Node<u32>,

  pub ray: ShaderRay,
  pub range: ShaderRayRange,

  pub payload: Node<i32>,
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

pub type RayAnyHitBehavior = u32;
pub const ACCEPT_HIT: RayAnyHitBehavior = 1;
pub const TERMINATE_TRAVERSE: RayAnyHitBehavior = 2;

pub type RayHitKind = u32;
pub const HIT_KIND_FRONT_FACING_TRIANGLE: RayHitKind = 0xFE;
pub const HIT_KIND_BACK_FACING_TRIANGLE: RayHitKind = 0xFF;

// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkGeometryFlagBitsKHR.html
pub type GeometryFlags = u32;
/// this geometry does not invoke the any-hit shaders even if present in a hit group.
pub const GEOMETRY_FLAG_OPAQUE: GeometryFlags = 0x1;
/// the implementation must only call the any-hit shader a single time for each primitive in this geometry. If this bit is absent an implementation may invoke the any-hit shader more than once for this geometry.
pub const GEOMETRY_FLAG_NO_DUPLICATE_ANYHIT_INVOCATION: GeometryFlags = 0x2;

// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkGeometryInstanceFlagBitsNV.html
pub type GeometryInstanceFlags = u32;
/// disables face culling for this instance.
pub const GEOMETRY_INSTANCE_TRIANGLE_FACING_CULL_DISABLE: GeometryInstanceFlags = 0x1;
/// the facing determination for geometry in this instance is inverted.
pub const GEOMETRY_INSTANCE_TRIANGLE_FLIP_FACING: GeometryInstanceFlags = 0x2;
/// this instance to act as though VK_GEOMETRY_OPAQUE_BIT_KHR were specified on all geometries referenced by this instance. This behavior can be overridden by the SPIR-V NoOpaqueKHR ray flag.
pub const GEOMETRY_INSTANCE_FORCE_OPAQUE: GeometryInstanceFlags = 0x4;
/// causes this instance to act as though VK_GEOMETRY_OPAQUE_BIT_KHR were not specified on all geometries referenced by this instance. This behavior can be overridden by the SPIR-V OpaqueKHR ray flag.
pub const GEOMETRY_INSTANCE_FORCE_NO_OPAQUE: GeometryInstanceFlags = 0x8;
