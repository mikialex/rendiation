use crate::*;

struct DeviceMapCompute<I, O> {
  mapper: Arc<dyn Fn(I) -> O>,
  mapper_extra_hasher: Arc<dyn ShaderHashProviderAny>,
  upstream: Box<dyn DeviceInvocationComponent<I>>,
}

impl<I, O> ShaderHashProvider for DeviceMapCompute<I, O> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::any::type_name_of_val(&self.mapper).hash(hasher);
    self
      .mapper_extra_hasher
      .hash_pipeline_with_type_info(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
}

impl<I: 'static, O: 'static + Copy> DeviceInvocationComponent<O> for DeviceMapCompute<I, O> {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<O>> {
    let source = self.upstream.build_shader(builder);

    let r = builder.entry_by(|cx| {
      let (input, valid) = source.invocation_logic(cx.global_invocation_id());

      let output = (self.mapper)(input);

      (output, valid)
    });

    source.get_size_into_adhoc(r).into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    self.upstream.requested_workgroup_size()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceMap<I, O> {
  pub mapper: Arc<dyn Fn(I) -> O>,
  pub mapper_extra_hasher: Arc<dyn ShaderHashProviderAny>,
  pub upstream: Box<dyn DeviceParallelCompute<I>>,
}

impl<I: 'static, O: Copy + 'static> DeviceParallelCompute<O> for DeviceMap<I, O> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<O>> {
    Box::new(DeviceMapCompute {
      mapper: self.mapper.clone(),
      upstream: self.upstream.execute_and_expose(cx),
      mapper_extra_hasher: self.mapper_extra_hasher.clone(),
    })
  }

  fn work_size(&self) -> u32 {
    self.upstream.work_size()
  }
}
impl<I: 'static, O: Copy + 'static> DeviceParallelComputeIO<O> for DeviceMap<I, Node<O>> {}

#[pollster::test]
async fn test() {
  let input = vec![1_u32; 70];

  let expect = input.iter().map(|v| v + 1).collect();

  input.map(|v| v + val(1)).single_run_test(&expect).await
}
