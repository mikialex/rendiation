use crate::*;

pub trait ShaderFutureProvider {
  type Output;
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder {
  type Output;
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> Self::Output;
  fn bind(&self, builder: &mut BindingBuilder);
}

pub trait NativeRayTracingShaderCtx {
  fn binding_builder(&mut self) -> &mut ShaderBindGroupBuilder;
  fn native_trace_ray(&self, ray: ShaderRayTraceCall, payload: Box<dyn Any>);
  fn tracing_ctx(&mut self) -> &mut TracingCtx;
}

pub trait TraceOperator<T>:
  ShaderFutureProvider<Output = T>
  + NativeRayTracingShaderBuilder<Output = T>
  + ShaderHashProvider
  + DynClone
  + 'static
{
}

impl<O, T> TraceOperator<O> for T where
  T: ShaderFutureProvider<Output = O>
    + NativeRayTracingShaderBuilder<Output = O>
    + ShaderHashProvider
    + DynClone
    + 'static
{
}

impl<O> NativeRayTracingShaderBuilder for Box<dyn TraceOperator<O>> {
  type Output = O;
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O {
    (**self).build(ctx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    (**self).bind(builder)
  }
}

impl<O> ShaderFutureProvider for Box<dyn TraceOperator<O>> {
  type Output = O;
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<O> {
    (**self).build_device_future(ctx)
  }
}

impl<O: 'static> ShaderHashProvider for Box<dyn TraceOperator<O>> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }
}

impl<T> Clone for Box<dyn TraceOperator<T>> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
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
