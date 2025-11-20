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

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceComputeZip<A, B> {
  pub source_a: Box<dyn DeviceInvocationComponent<A>>,
  pub source_b: Box<dyn DeviceInvocationComponent<B>>,
}

impl<A: 'static, B: 'static> ShaderHashProvider for DeviceComputeZip<A, B> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.source_a.hash_pipeline_with_type_info(hasher);
    self.source_b.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<A: 'static, B: 'static> DeviceInvocationComponent<(A, B)> for DeviceComputeZip<A, B> {
  fn result_size(&self) -> u32 {
    self.source_a.result_size().min(self.source_b.result_size())
  }

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

  fn work_size(&self) -> Option<u32> {
    self
      .source_a
      .work_size()
      .zip(self.source_b.work_size())
      .map(|(a, b)| a.min(b))
  }

  fn clone_boxed(&self) -> Box<dyn DeviceInvocationComponent<(A, B)>> {
    Box::new(self.clone())
  }
}

#[pollster::test]
async fn test() {
  gpu_cx!(cx);
  let input = vec![1_u32; 70];
  let input2 = vec![1_u32; 70];

  let expect = vec![2_u32; 70];

  let input = slice_into_compute(&input, cx);
  let input2 = slice_into_compute(&input2, cx);

  input
    .zip(input2)
    .map(|(a, b)| a + b)
    .run_test(cx, &expect)
    .await;
}
