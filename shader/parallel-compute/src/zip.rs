use crate::*;

pub struct DeviceInvocationZip<A, B>(
  pub Box<dyn DeviceInvocation<A>>,
  pub Box<dyn DeviceInvocation<B>>,
);

impl<A, B> DeviceInvocation<(A, B)> for DeviceInvocationZip<A, B> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> ((A, B), Node<bool>) {
    let left = self.0.invocation_logic(logic_global_id);
    let right = self.1.invocation_logic(logic_global_id);
    ((left.0, right.0), left.1.and(right.1))
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.0.invocation_size().min(self.1.invocation_size())
  }
}

struct Builder<A, B> {
  pub source_a: Box<dyn DeviceInvocationComponent<A>>,
  pub source_b: Box<dyn DeviceInvocationComponent<B>>,
}

impl<A: 'static, B: 'static> ShaderHashProvider for Builder<A, B> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.source_a.hash_pipeline_with_type_info(hasher);
    self.source_b.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<A: 'static, B: 'static> DeviceInvocationComponent<(A, B)> for Builder<A, B> {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<(A, B)>> {
    Box::new(DeviceInvocationZip(
      self.source_a.build_shader(builder),
      self.source_b.build_shader(builder),
    ))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.source_a.bind_input(builder);
    self.source_b.bind_input(builder);
  }
  fn requested_workgroup_size(&self) -> Option<u32> {
    assert_eq!(
      self.source_a.requested_workgroup_size(),
      self.source_b.requested_workgroup_size()
    );
    self.source_a.requested_workgroup_size()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceParallelComputeZip<A, B> {
  pub source_a: Box<dyn DeviceParallelCompute<A>>,
  pub source_b: Box<dyn DeviceParallelCompute<B>>,
  /// if we not add cache here, maybe work_size() will have exponential cost!
  pub work_size_cache: u32,
}

impl<A: 'static, B: 'static> DeviceParallelCompute<(A, B)> for DeviceParallelComputeZip<A, B> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<(A, B)>> {
    Box::new(Builder {
      source_a: self.source_a.execute_and_expose(cx),
      source_b: self.source_b.execute_and_expose(cx),
    })
  }

  fn work_size(&self) -> u32 {
    self.work_size_cache
  }
}

#[pollster::test]
async fn test() {
  let input = vec![1_u32; 70];
  let input2 = vec![1_u32; 70];

  let expect = vec![2_u32; 70];

  input
    .zip(input2)
    .map(|(a, b)| a + b)
    .single_run_test(&expect)
    .await
}
