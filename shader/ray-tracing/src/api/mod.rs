mod operator;
pub use operator::*;
mod ctx;
pub use ctx::*;
mod ty;
pub use ty::*;

use crate::*;

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder {
  type Ctx;
  fn build(&self, ctx: &mut Self::Ctx);
}
pub trait NativeRayTracingShaderCtx {
  fn native_trace_ray(&self, ray: ShaderRayTraceCall);
}

pub trait ShaderRayGenLogic:
  DeviceFuture<Ctx = RayGenShaderCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}

pub trait BoxShaderRayGenLogic {}

pub trait ShaderRayClosestHitLogic:
  DeviceFuture<Ctx = RayClosestHitCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}
pub trait BoxShaderRayClosestHitLogic {}

pub struct GPURaytracingPipelineBuilder {
  pub max_recursion_depth: u32,
  pub geometry_provider: Box<dyn GPUAccelerationStructureProvider>,
  ray_gen_shader: Box<dyn BoxShaderRayGenLogic>,
  // // miss_shader
  // miss_hit_shaders: Vec<Box<dyn BoxShaderRayClosestHitLogic>>,
  // intersection_shaders
  closest_hit_shaders: Vec<Box<dyn BoxShaderRayClosestHitLogic>>,
  // any_hit_shaders
}

impl Default for GPURaytracingPipelineBuilder {
  fn default() -> Self {
    Self {
      max_recursion_depth: 8,
      geometry_provider: todo!(),
      ray_gen_shader: todo!(),
      closest_hit_shaders: Default::default(),
    }
  }
}
pub struct GPURaytracingPipeline {
  pub internal: Box<dyn GPURaytracingPipelineProvider>,
}

pub enum RayAnyHitBehavior {
  IgnoreThisIntersect,
  TerminateTraverse,
}

impl GPURaytracingPipelineBuilder {
  pub fn with_max_recursion_depth(mut self, max_recursion_depth: u32) -> Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn with_ray_gen(self, ray_logic: impl ShaderRayGenLogic) -> Self {
    self
  }
  pub fn with_ray_intersection(
    self,
    range: usize,
    builder: impl FnOnce(&mut RayIntersectCtx),
  ) -> Self {
    self
  }

  pub fn with_ray_closest_hit(
    self,
    range: usize,
    ray_logic: impl ShaderRayClosestHitLogic,
  ) -> Self {
    self
  }

  pub fn with_ray_any_hit(
    self,
    range: usize,
    builder: impl FnOnce(&mut RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> Self {
    self
  }

  pub fn with_ray_miss(self, builder: impl FnOnce(&mut RayMissCtx)) -> Self {
    self
  }
}

// #[test]
// fn t() {
//   ray_ctx_from_declared_payload_input()
//     .then_trace_ray(|state, ctx| {
//       //
//     })
//     .then(|state, ctx| {
//       //
//     })
//     .then_trace_ray(|state, ctx| {
//       //
//     })
//     .then(|state, ctx| {
//       //
//     })
// }
