use crate::*;

pub trait GPUAccelerationStructureSystemCompImplInstance {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable>;
  fn bind_pass(&self, builder: &mut BindingBuilder);
}

pub trait GPUAccelerationStructureSystemCompImplInvocationTraversable {
  /// return optional closest hit
  fn traverse(
    &self,
    trace_payload: ENode<ShaderRayTraceCallStoragePayload>,
    intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> ShaderOption<RayClosestHitCtx>;
}

#[derive(Clone, Copy)]
pub struct ShaderOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}
