use crate::*;

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder<Cx> {
  fn build(&self, ctx: &mut Cx);
}
pub trait NativeRayTracingShaderCtx {
  fn native_trace_ray(&self, ray: ShaderRayTraceCall);
}
