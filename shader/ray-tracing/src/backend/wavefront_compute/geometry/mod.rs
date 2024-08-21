use crate::*;

pub mod naive;

pub trait GPUAccelerationStructureCompImplInstance {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureCompImplInvocationTraversable>;
  fn bind_pass(&self, pass: &mut GPUComputePass);

  fn handle(&self) -> u32;
}

pub trait GPUAccelerationStructureCompImplInvocationTraversable {
  /// return optional closest hit
  fn traverse(
    &self,
    intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> DeviceOption<HitInfo>;
}

#[derive(Clone, Copy)]
pub struct DeviceOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}
