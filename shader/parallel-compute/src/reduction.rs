use crate::*;

struct WorkGroupReductionCompute<T, S> {
  workgroup_size: u32,
  reduction_logic: PhantomData<S>,
  upstream: Box<dyn DeviceInvocationComponent<Node<T>>>,
}

impl<T, S> ShaderHashProvider for WorkGroupReductionCompute<T, S> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.workgroup_size.hash(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
}

impl<T, S> DeviceInvocationComponent<Node<T>> for WorkGroupReductionCompute<T, S>
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
    let source = self.upstream.build_shader(builder);

    let (result, valid) = builder.entry_by(|cx| {
      let (input, valid) = source.invocation_logic(cx.global_invocation_id());

      let input = valid.select(input, S::identity());

      let shared = cx.define_workgroup_shared_var_host_size_array::<T>(self.workgroup_size);

      let local_id = cx.local_invocation_id().x();

      shared.index(local_id).store(input);

      let iter = self.workgroup_size.ilog2();

      iter.into_shader_iter().for_each(|i, _| {
        cx.workgroup_barrier();

        let stride = val(1) << (val(iter) - i);

        if_by(local_id.less_than(stride), || {
          let a = shared.index(local_id).load();
          let b = shared.index(local_id - (val(1) << i)).load();
          let combined = S::combine(a, b);
          shared.index(local_id).store(combined);
        });

        cx.workgroup_barrier();
      });

      let result = local_id
        .equals(0)
        .select_branched(|| shared.index(0).load(), || S::identity());

      (result, local_id.equals(0))
    });

    Box::new(AdhocInvocationResult(result, valid))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct WorkGroupReduction<T, S> {
  pub workgroup_size: u32,
  pub reduction_logic: PhantomData<S>,
  pub upstream: Box<dyn DeviceParallelComputeIO<T>>,
}

impl<T, S> DeviceParallelCompute<Node<T>> for WorkGroupReduction<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    Box::new(WorkGroupReductionCompute::<T, S> {
      workgroup_size: self.workgroup_size,
      upstream: self.upstream.execute_and_expose(cx),
      reduction_logic: Default::default(),
    })
  }

  fn work_size(&self) -> u32 {
    self.upstream.work_size()
  }
}

impl<T, S> DeviceParallelComputeIO<T> for WorkGroupReduction<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceMonoidLogic<Data = T> + 'static,
{
  fn result_size(&self) -> u32 {
    self.work_size() / self.workgroup_size
  }

  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    let workgroup_size = self.workgroup_size;
    custom_write_into_storage_buffer(self, cx, move |global_id| global_id / val(workgroup_size))
  }
}
