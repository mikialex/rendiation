mod operator;
pub use operator::*;
mod ctx;
pub use ctx::*;

use crate::*;

pub trait ShaderFuture {
  type State;
  type Output;
  type Ctx;
  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State;
  fn device_poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> (Node<bool>, Self::Output);
}

pub trait RayTracingShaderBuilderWithNativeSupport {
  type Ctx;
  fn build(&self, ctx: &mut Self::Ctx) -> Self;
}

pub trait ShaderRayGenLogic:
  ShaderFuture<Ctx = RayGenShaderCtx, Output = ShaderRayTraceCall>
  + RayTracingShaderBuilderWithNativeSupport<Ctx = RayGenShaderCtx>
{
}
pub trait ShaderRayClosestHitLogic:
  ShaderFuture<Ctx = RayClosestHitCtx, Output = ShaderRayTraceCall>
  + RayTracingShaderBuilderWithNativeSupport<Ctx = RayClosestHitCtx>
{
}

pub struct GPURaytracingPipelineBuilder {}
pub struct GPURaytracingPipeline {
  pub internal: Box<dyn GPURaytracingPipelineProvider>,
}

impl GPURaytracingPipelineBuilder {
  pub fn with_ray_gen(self, ray_logic: impl ShaderRayGenLogic) -> Self {
    self
  }
  pub fn with_ray_intersection(self, builder: impl FnOnce(&mut usize)) -> Self {
    self
  }

  pub fn with_ray_closest_hit(
    self,
    range: usize,
    ray_logic: impl ShaderRayClosestHitLogic,
  ) -> Self {
    self
  }

  pub fn with_ray_any_hit(self, range: usize, builder: impl FnOnce(&mut usize)) -> Self {
    self
  }

  pub fn with_ray_miss(self, builder: impl FnOnce(&mut usize)) -> Self {
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
