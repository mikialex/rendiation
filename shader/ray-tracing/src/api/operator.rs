use crate::*;

pub trait ShaderFutureProvider<T> {
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<T>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder<O> {
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O;
  fn bind(&self, builder: &mut BindingBuilder);
}

pub trait NativeRayTracingShaderCtx {
  fn binding_builder(&mut self) -> &mut ShaderBindGroupBuilder;
  fn native_trace_ray(&self, ray: ShaderRayTraceCall, payload: Box<dyn Any>);
  fn tracing_ctx(&mut self) -> &mut TracingCtx;
}

pub trait TraceOperator<T>: ShaderFutureProvider<T> + NativeRayTracingShaderBuilder<T> {}
impl<O, T> TraceOperator<O> for T where T: ShaderFutureProvider<O> + NativeRayTracingShaderBuilder<O>
{}

impl<O> NativeRayTracingShaderBuilder<O> for Box<dyn TraceOperator<O>> {
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O {
    (**self).build(ctx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    (**self).bind(builder)
  }
}

impl<O> ShaderFutureProvider<O> for Box<dyn TraceOperator<O>> {
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<O> {
    (**self).build_device_future(ctx)
  }
}

pub trait RayCtxBaseProvider {
  /// the implementor should register proper ctx for miss-hit shader stage
  fn miss_shader_base(
    &self,
    payload_desc: ShaderSizedValueType,
  ) -> Box<dyn TraceOperator<Node<AnyType>>>;
  /// the implementor should register proper ctx for closest-hit shader stage
  fn closest_shader_base(
    &self,
    payload_desc: ShaderSizedValueType,
  ) -> Box<dyn TraceOperator<Node<AnyType>>>;
}
