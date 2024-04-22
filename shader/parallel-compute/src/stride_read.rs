use crate::*;

struct DeviceInvocationStride<T>(Box<dyn DeviceInvocation<T>>, Vec3<u32>);

impl<T> DeviceInvocation<T> for DeviceInvocationStride<T> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>) {
    let logic_global_id = logic_global_id * val(self.1);
    self.0.invocation_logic(logic_global_id)
  }
}

struct Builder<T> {
  pub source: Box<dyn DeviceInvocationComponent<T>>,
  pub stride: Vec3<u32>,
}

impl<T> ShaderHashProvider for Builder<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.stride.hash(hasher);
    self.source.hash_pipeline_with_type_info(hasher)
  }
}

impl<T: 'static> DeviceInvocationComponent<T> for Builder<T> {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>> {
    Box::new(DeviceInvocationStride(
      self.source.build_shader(builder),
      self.stride,
    ))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.source.bind_input(builder);
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceParallelComputeStrideRead<T> {
  pub source: Box<dyn DeviceParallelCompute<T>>,
  pub stride: Vec3<u32>,
}

impl<T: 'static> DeviceParallelCompute<T> for DeviceParallelComputeStrideRead<T> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<T>> {
    Box::new(Builder {
      source: self.source.execute_and_expose(cx),
      stride: self.stride,
    })
  }

  fn work_size(&self) -> u32 {
    self.source.work_size() / (self.stride.x + self.stride.y + self.stride.z)
  }
}
impl<T: 'static> DeviceParallelComputeIO<T> for DeviceParallelComputeStrideRead<Node<T>> {}
