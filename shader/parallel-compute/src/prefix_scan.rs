use std::ops::Add;

use crate::*;

pub trait DeviceMonoidLogic {
  type Data: ShaderSizedValueNodeType;
  fn identity() -> Node<Self::Data>;
  fn combine(a: Node<Self::Data>, b: Node<Self::Data>) -> Node<Self::Data>;
}

#[derive(Default)]
pub struct AdditionMonoid<T>(PhantomData<T>);

impl<T> DeviceMonoidLogic for AdditionMonoid<T>
where
  T: PrimitiveShaderNodeType + ShaderSizedValueNodeType,
  Node<T>: Add<Node<T>, Output = Node<T>>,
  T: Zero,
{
  type Data = T;
  fn identity() -> Node<T> {
    val(T::zero())
  }

  fn combine(a: Node<T>, b: Node<T>) -> Node<T> {
    a + b
  }
}

struct WorkGroupPrefixScanKoggeStoneCompute<T, S> {
  workgroup_size: u32,
  scan_logic: PhantomData<S>,
  upstream: Box<dyn DeviceInvocationComponent<Node<T>>>,
}

impl<T: 'static, S: 'static> ShaderHashProvider for WorkGroupPrefixScanKoggeStoneCompute<T, S> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.workgroup_size.hash(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T, S> DeviceInvocationComponent<Node<T>> for WorkGroupPrefixScanKoggeStoneCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    Some(self.workgroup_size)
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let local_id = builder.local_invocation_id().x();
    let shared = builder.define_workgroup_shared_var_host_size_array::<T>(self.workgroup_size);

    let iter = self.workgroup_size.ilog2();

    self
      .upstream
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |upstream, id| {
        let (input, valid) = upstream.invocation_logic(id);
        let (input, valid) = (valid.select(input, S::identity()), valid);

        let value = input.make_local_var();

        shared.index(local_id).store(value.load());

        iter.into_shader_iter().for_each(|i, _| {
          workgroup_barrier();

          if_by(local_id.greater_equal_than(val(1) << i), || {
            let a = value.load();
            let b = shared.index(local_id - (val(1) << i)).load();
            let combined = S::combine(a, b);
            value.store(combined)
          });

          workgroup_barrier();
          shared.index(local_id).store(value.load())
        });

        (value.load(), valid)
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }

  fn work_size(&self) -> Option<u32> {
    self.upstream.work_size()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct WorkGroupPrefixScanKoggeStone<T, S> {
  pub workgroup_size: u32,
  pub scan_logic: PhantomData<S>,
  pub upstream: Box<dyn DeviceParallelComputeIO<T>>,
}

impl<T, S> DeviceParallelCompute<Node<T>> for WorkGroupPrefixScanKoggeStone<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(WorkGroupPrefixScanKoggeStoneCompute::<T, S> {
      workgroup_size: self.workgroup_size,
      upstream: self.upstream.execute_and_expose(cx),
      scan_logic: Default::default(),
    })
  }

  fn result_size(&self) -> u32 {
    self.upstream.result_size()
  }
}
impl<T, S> DeviceParallelComputeIO<T> for WorkGroupPrefixScanKoggeStone<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
}

#[pollster::test]
async fn test_workgroup_prefix_sum_kogge_stone() {
  let input = vec![1_u32; 70];

  let workgroup_size = 32;

  let mut expect = Vec::new();

  let mut local_idx = 0;
  let mut sum = 0;
  for i in input.clone() {
    if local_idx >= workgroup_size {
      local_idx = 0;
      sum = 0;
    }

    sum += i;
    expect.push(sum);

    local_idx += 1;
  }

  input
    .workgroup_scope_prefix_scan_kogge_stone::<AdditionMonoid<_>>(workgroup_size)
    .run_test(&expect)
    .await
}

#[pollster::test]
async fn test_prefix_sum_kogge_stone() {
  let input = vec![1_u32; 70];

  let workgroup_size = 32;

  let mut expect = Vec::new();

  let mut sum = 0;
  for i in input.clone() {
    sum += i;
    expect.push(sum);
  }

  input
    .segmented_prefix_scan_kogge_stone::<AdditionMonoid<_>>(workgroup_size, workgroup_size)
    .run_test(&expect)
    .await
}
