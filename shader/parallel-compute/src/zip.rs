use crate::*;

pub struct DeviceInvocationZip<A, B>(Box<dyn DeviceInvocation<A>>, Box<dyn DeviceInvocation<B>>);

impl<A, B> DeviceInvocation<(A, B)> for DeviceInvocationZip<A, B> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> ((A, B), Node<bool>) {
    let left = self.0.invocation_logic(logic_global_id);
    let right = self.1.invocation_logic(logic_global_id);
    ((left.0, right.0), left.1.and(right.1))
  }
}

struct Builder<A, B> {
  pub source_a: Box<dyn DeviceInvocationBuilder<A>>,
  pub source_b: Box<dyn DeviceInvocationBuilder<B>>,
}

impl<A, B> ShaderHashProvider for Builder<A, B> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.source_a.hash_pipeline_with_type_info(hasher);
    self.source_b.hash_pipeline_with_type_info(hasher)
  }
}

impl<A: 'static, B: 'static> DeviceInvocationBuilder<(A, B)> for Builder<A, B> {
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
}

pub struct DeviceParallelComputeZip<A, B> {
  pub source_a: Box<dyn DeviceParallelCompute<A>>,
  pub source_b: Box<dyn DeviceParallelCompute<B>>,
}

impl<A: 'static, B: 'static> DeviceParallelCompute<(A, B)> for DeviceParallelComputeZip<A, B> {
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<(A, B)>> {
    Box::new(Builder {
      source_a: self.source_a.compute_result(cx),
      source_b: self.source_b.compute_result(cx),
    })
  }

  fn work_size(&self) -> u32 {
    // i think this is actually intersection?
    self.source_a.work_size().min(self.source_b.work_size())
  }
}
