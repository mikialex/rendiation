use crate::*;

struct DeviceMapCompute<I, O> {
  mapper: fn(I) -> O,
  upstream: Box<dyn DeviceInvocationBuilder<I>>,
}

impl<I, O> ShaderHashProvider for DeviceMapCompute<I, O> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::any::type_name_of_val(&self.mapper).hash(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
}

impl<I: 'static, O: 'static + Copy> DeviceInvocationBuilder<O> for DeviceMapCompute<I, O> {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<O>> {
    let source = self.upstream.build_shader(builder);

    let (result, valid) = builder.entry_by(|cx| {
      let (input, valid) = source.invocation_logic(cx.global_invocation_id());

      let output = (self.mapper)(input);

      (output, valid)
    });

    Box::new(AdhocInvocationResult(result, valid))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder);
  }
}

pub struct DeviceMap<I, O> {
  pub mapper: fn(I) -> O,
  pub upstream: Box<dyn DeviceParallelCompute<I>>,
}

impl<I: 'static, O: Copy + 'static> DeviceParallelCompute<O> for DeviceMap<I, O> {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<O>> {
    Box::new(DeviceMapCompute {
      mapper: self.mapper,
      upstream: self.upstream.compute_result(cx),
    })
  }

  fn work_size(&self) -> u32 {
    self.upstream.work_size()
  }
}
impl<I: 'static, O: Copy + 'static> DeviceParallelComputeIO<O> for DeviceMap<I, Node<O>> {}
