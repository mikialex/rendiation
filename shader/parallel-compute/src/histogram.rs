use std::num::NonZeroU64;

use crate::*;

pub trait DeviceHistogramMappingLogic {
  type Data: ShaderSizedValueNodeType;
  const MAX: u32;
  fn map(data: Node<Self::Data>) -> Node<u32>;
}

struct WorkGroupHistogramCompute<T, S> {
  workgroup_size: u32,
  histogram_logic: PhantomData<S>,
  upstream: Box<dyn DeviceInvocationComponent<Node<T>>>,
}

impl<T: 'static, S: 'static> ShaderHashProvider for WorkGroupHistogramCompute<T, S>
where
  S: DeviceHistogramMappingLogic<Data = T>,
{
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.workgroup_size.hash(hasher);
    S::MAX.hash(hasher);
    self.upstream.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}

impl<T, S> DeviceInvocationComponent<Node<u32>> for WorkGroupHistogramCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn work_size(&self) -> Option<u32> {
    self.upstream.work_size()
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    Some(self.workgroup_size)
  }
  // todo, fix out bound access
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let local_id = builder.local_invocation_id().x();
    let shared =
      builder.define_workgroup_shared_var_host_size_array::<DeviceAtomic<u32>>(self.workgroup_size);

    self
      .upstream
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |upstream, id| {
        let (input, valid) = upstream.invocation_logic(id);

        if_by(valid, || {
          let target = S::map(input);
          shared.index(target).atomic_add(val(1));
        });

        workgroup_barrier();

        let output_valid = local_id.less_than(S::MAX);
        let result =
          output_valid.select_branched(|| shared.index(local_id).atomic_load(), || val(0));
        (result, output_valid)
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.upstream.bind_input(builder)
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct WorkGroupHistogram<T, S> {
  pub workgroup_size: u32,
  pub logic: PhantomData<S>,
  pub upstream: Box<dyn DeviceParallelComputeIO<T>>,
}

impl<T, S> WorkGroupHistogram<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn compute_result_typed(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> WorkGroupHistogramCompute<T, S> {
    WorkGroupHistogramCompute::<T, S> {
      workgroup_size: self.workgroup_size,
      upstream: self.upstream.execute_and_expose(cx),
      histogram_logic: Default::default(),
    }
  }
}

impl<T, S> DeviceParallelCompute<Node<u32>> for WorkGroupHistogram<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    Box::new(self.compute_result_typed(cx))
  }

  fn result_size(&self) -> u32 {
    self.upstream.result_size() / self.workgroup_size * S::MAX
  }
}
impl<T, S> DeviceParallelComputeIO<u32> for WorkGroupHistogram<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn materialize_storage_buffer_into(
    &self,
    target: StorageBufferDataView<[u32]>,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<u32> {
    let workgroup_size = self.workgroup_size;

    struct HistogramWrite(u32);
    impl ShaderHashProvider for HistogramWrite {
      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.0.hash(hasher);
      }
      shader_hash_type_id! {}
    }

    custom_write_into_storage_buffer(
      self,
      cx,
      move |global_id| global_id / val(workgroup_size * S::MAX) + global_id % val(workgroup_size),
      Box::new(HistogramWrite(workgroup_size)),
      target,
    )
  }
}

pub struct DeviceHistogramCompute<T, S> {
  workgroup_level: WorkGroupHistogramCompute<T, S>,
  result: StorageBufferDataView<[DeviceAtomic<u32>]>,
}

impl<T: 'static, S> ShaderHashProvider for DeviceHistogramCompute<T, S>
where
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.workgroup_level.hash_pipeline(hasher)
  }
  shader_hash_type_id! {}
}

impl<T, S> DeviceInvocationComponent<Node<u32>> for DeviceHistogramCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.workgroup_level.requested_workgroup_size()
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let local_id = builder.local_invocation_id().x();
    let result = builder.bind_by(&self.result);

    self
      .workgroup_level
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |workgroup_level, id| {
        let workgroup_level_histogram = workgroup_level.invocation_logic(id);
        let histogram_idx = local_id;

        result
          .index(histogram_idx)
          .expose()
          .atomic_store(workgroup_level_histogram.0);

        workgroup_level_histogram
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.result);
    self.workgroup_level.bind_input(builder);
  }

  fn work_size(&self) -> Option<u32> {
    self.workgroup_level.work_size()
  }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DeviceHistogram<T, S> {
  pub workgroup_level: WorkGroupHistogram<T, S>,
}

impl<T, S> DeviceHistogram<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn create_compute_instance_impl(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceHistogramCompute<T, S> {
    let workgroup_level = self.workgroup_level.compute_result_typed(cx);

    let size = NonZeroU64::new(S::MAX as u64 * std::mem::size_of::<T>() as u64).unwrap();
    let result = create_gpu_read_write_storage(StorageBufferInit::Zeroed(size), &cx.gpu.device);

    DeviceHistogramCompute::<T, S> {
      workgroup_level,
      result,
    }
  }
}

impl<T, S> DeviceParallelCompute<Node<u32>> for DeviceHistogram<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    Box::new(self.create_compute_instance_impl(cx))
  }

  fn result_size(&self) -> u32 {
    S::MAX
  }
}
impl<T, S> DeviceParallelComputeIO<u32> for DeviceHistogram<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<u32>
  where
    u32: Std430 + ShaderSizedValueNodeType,
  {
    let compute_instance = self.create_compute_instance_impl(cx);
    compute_instance.dispatch_compute(cx);
    DeviceMaterializeResult::full_buffer(
      compute_instance
        .result
        .into_host_nonatomic_array()
        .into_readonly_view(),
    )
  }
}

#[pollster::test]
async fn test() {
  struct TestRangedU32;
  impl DeviceHistogramMappingLogic for TestRangedU32 {
    type Data = u32;

    const MAX: u32 = 6;

    fn map(data: Node<Self::Data>) -> Node<u32> {
      data
    }
  }

  let input = [0, 0, 1, 2, 3, 4, 5].to_vec();
  let expect = [2, 1, 1, 1, 1, 1].to_vec();

  input.histogram::<TestRangedU32>(32).run_test(&expect).await
}
