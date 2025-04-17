use crate::*;

struct DeviceInvocationStride<T>(Box<dyn DeviceInvocation<T>>, u32, bool);

impl<T> DeviceInvocation<T> for DeviceInvocationStride<T> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (T, Node<bool>) {
    let target = if self.2 {
      logic_global_id * val(Vec3::splat(self.1))
    } else {
      logic_global_id / val(Vec3::splat(self.1))
    };
    self.0.invocation_logic(target)
  }
  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.0.invocation_size()
  }
}

struct Builder<T> {
  pub source: Box<dyn DeviceInvocationComponent<T>>,
  pub stride: u32,
  pub reduce: bool,
}

impl<T: 'static> ShaderHashProvider for Builder<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.stride.hash(hasher);
    self.reduce.hash(hasher);
    self.source.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T: 'static> DeviceInvocationComponent<T> for Builder<T> {
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<T>> {
    Box::new(DeviceInvocationStride(
      self.source.build_shader(builder),
      self.stride,
      self.reduce,
    ))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.source.bind_input(builder);
  }
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.source.requested_workgroup_size()
  }

  fn work_size(&self) -> Option<u32> {
    let work_size = self.source.work_size()?;
    if self.reduce {
      work_size.div_ceil(self.stride)
    } else {
      work_size * self.stride
    }
    .into()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceParallelComputeStrideRead<T> {
  pub source: Box<dyn DeviceParallelCompute<T>>,
  pub stride: u32,
  pub reduce: bool,
}

impl<T: 'static> DeviceParallelCompute<T> for DeviceParallelComputeStrideRead<T> {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<T>> {
    Box::new(Builder {
      source: self.source.execute_and_expose(cx),
      stride: self.stride,
      reduce: self.reduce,
    })
  }

  fn result_size(&self) -> u32 {
    if self.reduce {
      self.source.result_size().div_ceil(self.stride)
    } else {
      self.source.result_size() * self.stride
    }
  }
}
impl<T: 'static> DeviceParallelComputeIO<T> for DeviceParallelComputeStrideRead<Node<T>> {}

#[pollster::test]
async fn test_reduce() {
  let input: Vec<_> = (0..6).flat_map(|_| (0..6)).collect();
  let expect = vec![0; 6];

  input.stride_reduce_result(6).run_test(&expect).await
}

#[pollster::test]
async fn test_expand() {
  let input: Vec<_> = (0..6).collect();
  let expect = (0..6)
    .flat_map(|v| std::iter::repeat_n(v, 6))
    .collect::<Vec<_>>();

  input.stride_expand_result(6).run_test(&expect).await
}
