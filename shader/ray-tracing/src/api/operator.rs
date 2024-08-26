use crate::*;

pub trait RayCtxBaseProvider {
  fn miss_shader_base(
    &self,
    payload_desc: ShaderSizedValueType,
  ) -> Box<dyn TraceOperator<Node<AnyType>>>;
  fn closest_shader_base(
    &self,
    payload_desc: ShaderSizedValueType,
  ) -> Box<dyn TraceOperator<(Node<AnyType>, RayClosestHitCtx)>>;
}

pub struct RayCtxBaseBuilder {
  pub inner: Box<dyn RayCtxBaseProvider>,
}

impl RayCtxBaseBuilder {
  pub fn miss_shader_base<T: ShaderSizedValueNodeType>(
    &self,
  ) -> impl TraceOperator<(Node<T>, RayMissCtx)> {
    self.inner.miss_shader_base(T::sized_ty()).map(|o| todo!())
  }
  pub fn closest_shader_base<T: ShaderSizedValueNodeType>(
    &self,
  ) -> impl TraceOperator<(Node<T>, RayClosestHitCtx)> {
    self.inner.miss_shader_base(T::sized_ty()).map(|o| todo!())
  }
}

pub trait DeviceFutureProvider<T> {
  fn build_device_future(&self) -> DynDeviceFuture<T>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder<O> {
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O;
  fn bind(&self, builder: &mut BindingBuilder);
}

pub trait NativeRayTracingShaderCtx {
  fn native_trace_ray(&self, ray: ShaderRayTraceCall, payload: Box<dyn Any>);
}

pub trait TraceOperator<T>: DeviceFutureProvider<T> + NativeRayTracingShaderBuilder<T> {}
impl<O, T> TraceOperator<O> for T where T: DeviceFutureProvider<O> + NativeRayTracingShaderBuilder<O>
{}

impl<O> NativeRayTracingShaderBuilder<O> for Box<dyn TraceOperator<O>> {
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O {
    (**self).build(ctx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    (**self).bind(builder)
  }
}

impl<O> DeviceFutureProvider<O> for Box<dyn TraceOperator<O>> {
  fn build_device_future(&self) -> DynDeviceFuture<O> {
    (**self).build_device_future()
  }
}
