use crate::*;

pub struct DeviceInvocationZipPartial<A, B, F>(
  pub Box<dyn DeviceInvocation<A>>,
  pub Box<dyn DeviceInvocation<Node<B>>>,
  pub F,
);

impl<A, B, F> DeviceInvocation<(A, Node<B>)> for DeviceInvocationZipPartial<A, B, F>
where
  B: ShaderNodeType,
  F: Fn() -> Node<B>,
{
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> ((A, Node<B>), Node<bool>) {
    let left = self.0.invocation_logic(logic_global_id);
    let right = self.1.invocation_logic(logic_global_id);
    let right_v = right.1.select(right.0, (self.2)());
    ((left.0, right_v), left.1)
  }
}

struct Builder<A, B, F: 'static> {
  pub source_a: Box<dyn DeviceInvocationComponent<A>>,
  pub source_b: Box<dyn DeviceInvocationComponent<Node<B>>>,
  pub b_fn: F,
}

impl<A, B, F> ShaderHashProvider for Builder<A, B, F> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.source_a.hash_pipeline_with_type_info(hasher);
    self.source_b.hash_pipeline_with_type_info(hasher);
    std::any::TypeId::of::<F>().hash(hasher);
  }
}

impl<A, B, F> DeviceInvocationComponent<(A, Node<B>)> for Builder<A, B, F>
where
  A: 'static,
  B: ShaderNodeType,
  F: 'static + Fn() -> Node<B> + Clone,
{
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<(A, Node<B>)>> {
    Box::new(DeviceInvocationZipPartial(
      self.source_a.build_shader(builder),
      self.source_b.build_shader(builder),
      self.b_fn.clone(),
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
#[derivative(Clone(bound = "F: Clone"))]
pub struct DeviceParallelComputeZipPartial<A, B, F> {
  pub source_a: Box<dyn DeviceParallelCompute<A>>,
  pub source_b: Box<dyn DeviceParallelComputeIO<B>>,
  pub b_fn: F,
}

impl<A, B, F> DeviceParallelCompute<(A, Node<B>)> for DeviceParallelComputeZipPartial<A, B, F>
where
  A: 'static,
  B: 'static + ShaderNodeType,
  F: Clone + 'static + Fn() -> Node<B>,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<(A, Node<B>)>> {
    Box::new(Builder {
      source_a: self.source_a.execute_and_expose(cx),
      source_b: self.source_b.execute_and_expose(cx),
      b_fn: self.b_fn.clone(),
    })
  }

  fn work_size(&self) -> u32 {
    assert!(self.source_a.work_size() >= self.source_b.work_size());
    self.source_a.work_size()
  }
}

#[pollster::test]
async fn test() {
  let input = vec![1_u32; 70];
  let input2 = vec![1_u32; 60];

  let expect = vec![2_u32; 70];

  input
    .zip_partial(input2, || val(1))
    .map(|(a, b)| a + b)
    .single_run_test(&expect)
    .await
}
