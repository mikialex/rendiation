use crate::*;

impl<Output, Cx> NativeRayTracingShaderBuilder<Cx> for BaseDeviceFuture<Output> {
  fn build(&self, _: &mut Cx) {}
}

pub struct TraceNextRay<F, T> {
  upstream: T,
  next_trace_logic: F,
}

impl<F, T, Cx> NativeRayTracingShaderBuilder<Cx> for TraceNextRay<F, T>
where
  T: NativeRayTracingShaderBuilder<Cx>,
  Cx: NativeRayTracingShaderCtx,
  F: FnOnce() -> (Node<bool>, ShaderRayTraceCall) + Copy,
{
  fn build(&self, ctx: &mut Cx) {
    self.upstream.build(ctx);

    let (r, c) = (self.next_trace_logic)();
    if_by(r, || {
      ctx.native_trace_ray(c);
    });
  }
}
