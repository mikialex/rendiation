use crate::*;

#[derive(Clone)]
pub struct ShaderFutureProviderIntoTraceOperator<T>(pub T);

impl<T: ShaderHashProvider> ShaderHashProvider for ShaderFutureProviderIntoTraceOperator<T> {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.0.hash_type_info(hasher)
  }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline(hasher)
  }
}

impl<T: ShaderFutureProvider> ShaderFutureProvider for ShaderFutureProviderIntoTraceOperator<T> {
  type Output = T::Output;

  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    self.0.build_device_future(ctx)
  }
}

impl<T: ShaderFutureProvider> NativeRayTracingShaderBuilder
  for ShaderFutureProviderIntoTraceOperator<T>
where
  T::Output: 'static,
{
  type Output = T::Output;

  // here we only demonstrate the impl structure, defer the impl to the future
  #[allow(unreachable_code)]
  #[allow(dead_code)]
  #[allow(unused_variables)]
  #[allow(clippy::diverging_sub_expression)]
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> T::Output {
    let mut any = AnyMap::default();
    let future = self.0.build_device_future(&mut any);

    let mut poll_build_ctx = todo!();
    let poller = future.build_poll(poll_build_ctx);

    let mut poll_ctx = todo!();
    let resolved = val(false).make_local_var();
    loop_by(|cx| {
      if_by(resolved.load(), || {
        cx.do_break();
      });
      let poll_result = poller.device_poll(poll_ctx);
    });
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    let mut any = AnyMap::default();
    let future = self.0.build_device_future(&mut any);
    let mut cx = DeviceTaskSystemBindCtx { binder: builder };
    future.bind_input(&mut cx);
  }
}
