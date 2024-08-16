use crate::*;

pub type DynDeviceFuture<T> =
  Box<dyn DeviceFuture<Output = T, Invocation = Box<dyn DeviceFutureInvocation<Output = T>>>>;

pub trait DeviceFutureProvider<T> {
  fn build_device_future(&self) -> DynDeviceFuture<T>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder<Cx, O> {
  fn build(&self, ctx: &mut Cx) -> O;
}

pub trait NativeRayTracingShaderCtx {
  fn native_trace_ray(&self, ray: ShaderRayTraceCall);
}
