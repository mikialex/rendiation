use crate::*;

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DataShuffleMovement<T> {
  pub source: Box<dyn DeviceParallelCompute<(Node<T>, Node<u32>)>>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceParallelCompute<Node<T>>
  for DataShuffleMovement<T>
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    let temp_result = self.materialize_storage_buffer(cx);
    Box::new(StorageBufferReadOnlyDataViewReadIntoShader(temp_result))
  }

  fn work_size(&self) -> u32 {
    self.source.work_size()
  }
}
impl<T: Std430 + ShaderSizedValueNodeType> DeviceParallelComputeIO<T> for DataShuffleMovement<T> {
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> StorageBufferReadOnlyDataView<[T]>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    let input = self.source.execute_and_expose(cx);
    let output = create_gpu_read_write_storage::<[T]>(self.work_size() as usize, &cx.gpu);

    let write = ShuffleWrite { input, output };

    write.dispatch_compute(self.work_size(), cx);

    write.output.into_readonly_view()
  }
}

pub struct ShuffleWrite<T: Std430> {
  pub input: Box<dyn DeviceInvocationComponent<(Node<T>, Node<u32>)>>,
  /// shuffle access require reading any position, so we need fully materialized result here
  pub output: StorageBufferDataView<[T]>,
}

impl<T: Std430> ShaderHashProvider for ShuffleWrite<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.input.hash_pipeline_with_type_info(hasher);
  }
}

impl<T> DeviceInvocationComponent<Node<T>> for ShuffleWrite<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.input.requested_workgroup_size()
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let input = self.input.build_shader(builder);

    let r = builder.entry_by(|cx| {
      let invocation_id = cx.local_invocation_id();
      let output = cx.bind_by(&self.output);

      let ((data, write_idx), valid) = input.invocation_logic(invocation_id);

      if_by(valid, || {
        output.index(write_idx).store(data);
      });

      (data, valid)
    });
    Box::new(AdhocInvocationResult(r.0, r.1))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.input.bind_input(builder);
    builder.bind(&self.output);
  }
}

pub struct ShuffleAccess<T: Std430> {
  /// shuffle access require reading any position, so we need fully materialized result here
  pub source: StorageBufferReadOnlyDataView<[T]>,
  pub shuffle_idx: Box<dyn DeviceInvocationComponent<Node<u32>>>,
}

impl<T: Std430> ShaderHashProvider for ShuffleAccess<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.shuffle_idx.hash_pipeline_with_type_info(hasher);
  }
}

impl<T> DeviceInvocationComponent<Node<T>> for ShuffleAccess<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.shuffle_idx.requested_workgroup_size()
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let shuffle_idx = self.shuffle_idx.build_shader(builder);

    let r = builder.entry_by(|cx| {
      let invocation_id = cx.local_invocation_id();
      let input = cx.bind_by(&self.source);

      let (read_idx, valid) = shuffle_idx.invocation_logic(invocation_id);

      let r = valid.select(input.index(read_idx).load(), zeroed_val());

      (r, valid)
    });
    Box::new(AdhocInvocationResult(r.0, r.1))
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.shuffle_idx.bind_input(builder);
    builder.bind(&self.source);
  }
}
