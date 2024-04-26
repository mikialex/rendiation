use crate::*;

struct DeviceInvocationOffset<T>(Box<dyn DeviceInvocation<Node<T>>>, u32);

impl<T: ShaderSizedValueNodeType> DeviceInvocation<Node<T>> for DeviceInvocationOffset<T> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<T>, Node<bool>) {
    let logic_global_id = logic_global_id + val(Vec3::new(self.1, 0, 0));
    let (v, is_valid) = self.0.invocation_logic(logic_global_id);

    let va = is_valid.select_branched(|| v, || self.0.border());
    (va, val(true))
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.0.invocation_size()
  }
}

struct Builder<T> {
  pub source: Box<dyn DeviceInvocationComponent<Node<T>>>,
  pub offset: u32,
}

impl<T> ShaderHashProvider for Builder<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.offset.hash(hasher);
    self.source.hash_pipeline_with_type_info(hasher)
  }
}

impl<T: ShaderSizedValueNodeType> DeviceInvocationComponent<Node<T>> for Builder<T> {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    Box::new(DeviceInvocationOffset(
      self.source.build_shader(builder),
      self.offset,
    ))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.source.bind_input(builder);
  }
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.source.requested_workgroup_size()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceParallelComputeOffsetRead<T> {
  pub source: Box<dyn DeviceParallelComputeIO<T>>,
  pub offset: u32,
}

impl<T: ShaderSizedValueNodeType> DeviceParallelCompute<Node<T>>
  for DeviceParallelComputeOffsetRead<T>
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(Builder {
      source: self.source.execute_and_expose(cx),
      offset: self.offset,
    })
  }

  fn work_size(&self) -> u32 {
    self.source.work_size()
  }
}
impl<T: ShaderSizedValueNodeType> DeviceParallelComputeIO<T>
  for DeviceParallelComputeOffsetRead<T>
{
}

#[pollster::test]
async fn test() {
  let input = [0, 1, 2, 3, 4, 5].to_vec();
  let expect = [3, 4, 5, 5, 5, 5].to_vec();

  input.offset_access(3).single_run_test(&expect).await
}
