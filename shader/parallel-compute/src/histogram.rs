use crate::*;

pub trait DeviceHistogramMappingLogic {
  type Data: ShaderSizedValueNodeType;
  const MAX: u32;
  // if the output index larger than MAX, the index will be clamped
  fn map(data: Node<Self::Data>) -> Node<u32>;
}

#[derive_where(Clone)]
pub struct WorkGroupHistogramCompute<T, S> {
  pub workgroup_size: u32,
  pub histogram_logic: PhantomData<S>,
  pub upstream: Box<dyn ComputeComponent<Node<T>>>,
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

impl<T, S> ComputeComponent<Node<u32>> for WorkGroupHistogramCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<u32>>> {
    Box::new(self.clone())
  }
  fn result_size(&self) -> u32 {
    self.upstream.result_size().div_ceil(self.workgroup_size) * S::MAX
  }

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
    let shared = builder.define_workgroup_shared_var_host_size_array::<DeviceAtomic<u32>>(S::MAX);

    self
      .upstream
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |upstream, id| {
        let output_valid = local_id.less_than(S::MAX);

        if_by(output_valid, || {
          // reset the shared memory(we assume zero_initialize_workgroup_memory is false)
          shared.index(local_id).atomic_store(val(0));
        });

        workgroup_barrier();

        let (input, valid) = upstream.invocation_logic(id);

        if_by(valid, || {
          let target = S::map(input);
          // clamp the write index if the map function produce invalid result
          let target = target.min(val(S::MAX - 1));
          shared.index(target).atomic_add(val(1));
        });

        workgroup_barrier();

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

impl<T, S> ComputeComponentIO<u32> for WorkGroupHistogramCompute<T, S>
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

    struct HistogramWrite(u32, u32);
    impl ShaderHashProvider for HistogramWrite {
      fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.0.hash(hasher);
        self.1.hash(hasher);
      }
      shader_hash_type_id! {}
    }

    custom_write_into_storage_buffer(
      self,
      cx,
      move |global_id| {
        let workgroup_index = global_id / val(workgroup_size);
        let local_index = global_id % val(workgroup_size);
        let valid = local_index.less_than(S::MAX);
        let idx = val(S::MAX) * workgroup_index + local_index;
        (idx, valid)
      },
      Arc::new(HistogramWrite(workgroup_size, S::MAX)),
      target,
    )
  }
}

#[derive_where(Clone)]
pub struct DeviceHistogramCompute<T, S> {
  pub workgroup_level: WorkGroupHistogramCompute<T, S>,
  pub result: StorageBufferDataView<[DeviceAtomic<u32>]>,
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

impl<T, S> ComputeComponent<Node<u32>> for DeviceHistogramCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  fn result_size(&self) -> u32 {
    S::MAX
  }

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

        if_by(workgroup_level_histogram.1, || {
          result
            .index(histogram_idx)
            .atomic_store(workgroup_level_histogram.0);
        });

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

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<u32>>> {
    Box::new(self.clone())
  }
}

impl<T, S> DeviceHistogramCompute<T, S>
where
  T: ShaderSizedValueNodeType,
  S: DeviceHistogramMappingLogic<Data = T> + 'static,
{
  pub fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<u32>
  where
    u32: Std430 + ShaderSizedValueNodeType,
  {
    self.dispatch_compute(cx);
    DeviceMaterializeResult::full_buffer(
      self
        .result
        .clone()
        .into_host_nonatomic_array()
        .into_readonly_view(),
    )
  }
}

#[pollster::test]
async fn test_histogram_workgroup() {
  gpu_cx!(cx);
  struct TestRangedU32;
  impl DeviceHistogramMappingLogic for TestRangedU32 {
    type Data = u32;

    const MAX: u32 = 4;

    fn map(data: Node<Self::Data>) -> Node<u32> {
      data
    }
  }

  let input = [0, 0, 1, 2, 3, 2, 1].to_vec();
  let expect = [2, 1, 1, 0, 0, 1, 1, 1].to_vec();
  let input = slice_into_compute(&input, cx);

  input
    .workgroup_histogram::<TestRangedU32>(4, cx)
    .run_test(cx, &expect)
    .await
}

#[pollster::test]
async fn test_histogram() {
  gpu_cx!(cx);
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
  let input = slice_into_compute(&input, cx);

  input
    .histogram::<TestRangedU32>(32, cx)
    .run_test(cx, &expect)
    .await
}

#[pollster::test]
async fn test_histogram_clamp_behavior() {
  gpu_cx!(cx);
  struct TestRangedU32;
  impl DeviceHistogramMappingLogic for TestRangedU32 {
    type Data = u32;

    const MAX: u32 = 4;

    fn map(data: Node<Self::Data>) -> Node<u32> {
      data
    }
  }

  let input = [0, 0, 1, 2, 3, 4, 5].to_vec();
  let expect = [2, 1, 1, 3].to_vec();
  let input = slice_into_compute(&input, cx);

  input
    .histogram::<TestRangedU32>(32, cx)
    .run_test(cx, &expect)
    .await
}
