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
