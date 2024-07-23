mod operator;
pub use operator::*;
mod ctx;
pub use ctx::*;

use crate::*;

#[derive(Clone, Copy)]
pub struct DeviceOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}

impl<T> From<(Node<bool>, T)> for DeviceOption<T> {
  fn from((is_some, payload): (Node<bool>, T)) -> Self {
    Self { is_some, payload }
  }
}

impl<T: Copy> DeviceOption<T> {
  pub fn some(payload: T) -> Self {
    Self {
      is_some: val(true),
      payload,
    }
  }

  pub fn map<U: ShaderSizedValueNodeType>(
    self,
    f: impl FnOnce(T) -> Node<U> + Copy,
  ) -> DeviceOption<Node<U>> {
    let u = zeroed_val().make_local_var();
    if_by(self.is_some, || u.store(f(self.payload)));
    (self.is_some, u.load()).into()
  }
}

pub trait ShaderFuture {
  type State;
  type Output;
  type Ctx;
  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State;
  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DeviceOption<Self::Output>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait RayTracingShaderBuilderWithNativeRayTracingSupport {
  type Ctx;
  fn build(&self, ctx: &mut Self::Ctx) -> Self;
}

pub trait ShaderRayGenLogic:
  ShaderFuture<Ctx = RayGenShaderCtx, Output = ShaderRayTraceCall>
  + RayTracingShaderBuilderWithNativeRayTracingSupport<Ctx = RayGenShaderCtx>
{
}

pub trait ShaderRayClosestHitLogic:
  ShaderFuture<Ctx = RayClosestHitCtx, Output = ShaderRayTraceCall>
  + RayTracingShaderBuilderWithNativeRayTracingSupport<Ctx = RayClosestHitCtx>
{
}

pub struct GPURaytracingPipelineBuilder {
  pub geometry_provider: Box<dyn GPUAccelerationStructureProvider>,
  // ray_gen_shader: Box<dyn ShaderRayGenLogic>,
  // miss_shader
  // intersection_shaders
  // closest_hit_shaders
  // any_hit_shaders
}
pub struct GPURaytracingPipeline {
  pub internal: Box<dyn GPURaytracingPipelineProvider>,
}

impl GPURaytracingPipelineBuilder {
  pub fn with_ray_gen(self, ray_logic: impl ShaderRayGenLogic) -> Self {
    self
  }
  pub fn with_ray_intersection(self, range: usize, builder: impl FnOnce(&mut usize)) -> Self {
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
