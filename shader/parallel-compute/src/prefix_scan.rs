use std::{marker::PhantomData, ops::Add};

use num_traits::One;

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
  T: One,
{
  type Data = T;
  fn identity() -> Node<T> {
    val(T::one())
  }

  fn combine(a: Node<T>, b: Node<T>) -> Node<T> {
    a + b
  }
}

struct WorkGroupPrefixScanKoggeStoneCompute<T, S> {
  workgroup_size: u32,
  scan_logic: PhantomData<S>,
  upstream: Box<dyn DeviceInvocationBuilder<Node<T>>>,
}

impl<T, S> ShaderHashProvider for WorkGroupPrefixScanKoggeStoneCompute<T, S> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.workgroup_size.hash(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
}

impl<T, S> DeviceInvocationBuilder<Node<T>> for WorkGroupPrefixScanKoggeStoneCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let source = self.upstream.build_shader(builder);

    let (result, valid) = builder.entry_by(|cx| {
      let (input, valid) = source.invocation_logic(cx.global_invocation_id());

      let input = valid.select(input, S::identity());

      let shared = cx.define_workgroup_shared_var_host_size_array::<T>(self.workgroup_size);

      let local_id = cx.local_invocation_id().x();

      let value = input.make_local_var();

      shared.index(local_id).store(value.load());

      self
        .workgroup_size
        .ilog2()
        .into_shader_iter()
        .for_each(|i, _| {
          cx.workgroup_barrier();

          if_by(local_id.greater_equal_than(val(1) << i), || {
            let a = value.load();
            let b = shared.index(local_id - (val(1) << i)).load();
            let combined = S::combine(a, b);
            value.store(combined)
          });

          cx.workgroup_barrier();
          shared.index(local_id).store(value.load())
        });

      (value.load(), valid)
    });

    Box::new(AdhocInvocationResult(result, valid))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }
}

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
  fn compute_result(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationBuilder<Node<T>>> {
    Box::new(WorkGroupPrefixScanKoggeStoneCompute::<T, S> {
      workgroup_size: self.workgroup_size,
      upstream: self.upstream.compute_result(cx),
      scan_logic: Default::default(),
    })
  }

  fn work_size(&self) -> u32 {
    self.upstream.work_size()
  }
}
impl<T, S> DeviceParallelComputeIO<T> for WorkGroupPrefixScanKoggeStone<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
}
