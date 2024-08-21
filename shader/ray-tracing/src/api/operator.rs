use crate::*;

pub trait DeviceFutureProvider<T> {
  fn build_device_future(&self) -> DynDeviceFuture<T>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder<Cx, O> {
  fn build(&self, ctx: &mut Cx) -> O;
  fn bind(&self, builder: &mut BindingBuilder);
}

pub trait NativeRayTracingShaderCtx {
  fn native_trace_ray(&self, ray: ShaderRayTraceCall, payload: Box<dyn Any>);
}
