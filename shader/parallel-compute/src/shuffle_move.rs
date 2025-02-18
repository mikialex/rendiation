use crate::*;

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct DataShuffleMovement<T> {
  pub source: Box<dyn DeviceParallelCompute<(Node<T>, Node<u32>, Node<bool>)>>,
}

impl<T: Std430 + ShaderSizedValueNodeType> DeviceParallelCompute<Node<T>>
  for DataShuffleMovement<T>
{
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<T>>> {
    self.materialize_storage_buffer(cx).into_boxed()
  }
  fn result_size(&self) -> u32 {
    self.source.result_size()
  }
}
impl<T: Std430 + ShaderSizedValueNodeType> DeviceParallelComputeIO<T> for DataShuffleMovement<T> {
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    let input = self.source.execute_and_expose(cx);
    let output = create_gpu_read_write_storage::<[T]>(self.result_size() as usize, &cx.gpu);

    let write = ShuffleWrite {
      input,
      output: output.clone(),
    };

    // should size be the atomic max of the shuffle destination?
    let size = write.dispatch_compute(cx);
    DeviceMaterializeResult {
      buffer: output.into_readonly_view(),
      size,
    }
  }
}

pub struct ShuffleWrite<T: Std430> {
  pub input: Box<dyn DeviceInvocationComponent<(Node<T>, Node<u32>, Node<bool>)>>,
  /// shuffle access require reading any position, so we need fully materialized result here
  pub output: StorageBufferDataView<[T]>,
}

impl<T: Std430> ShaderHashProvider for ShuffleWrite<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.input.hash_pipeline_with_type_info(hasher);
  }
  shader_hash_type_id! {}
}

impl<T> DeviceInvocationComponent<Node<T>> for ShuffleWrite<T>
where
  T: Std430 + ShaderSizedValueNodeType + SizedShaderValueAbstractPtrAccess<ShaderNodeRawHandle>,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.input.requested_workgroup_size()
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let output = builder.bind_by(&self.output);
    self
      .input
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |input, id| {
        let ((data, write_idx, should_write), valid) = input.invocation_logic(id);
        let write_is_in_bound = write_idx.less_than(output.array_length());
        if_by(valid.and(should_write).and(write_is_in_bound), || {
          output.index(write_idx).store(data);
        });

        (data, valid)
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.output);
    self.input.bind_input(builder);
  }

  fn work_size(&self) -> Option<u32> {
    self.input.work_size()
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
  shader_hash_type_id! {}
}

impl<T> DeviceInvocationComponent<Node<T>> for ShuffleAccess<T>
where
  T: Std430 + ShaderSizedValueNodeType + SizedShaderValueAbstractPtrAccess<ShaderNodeRawHandle>,
{
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.shuffle_idx.requested_workgroup_size()
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<T>>> {
    let input = builder.bind_by(&self.source);

    self
      .shuffle_idx
      .build_shader(builder)
      .adhoc_invoke_with_self_size(move |shuffle_idx, id| {
        let (read_idx, valid) = shuffle_idx.invocation_logic(id);

        let r = valid.select(input.index(read_idx).load(), zeroed_val());

        (r, valid)
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.shuffle_idx.bind_input(builder);
    builder.bind(&self.source);
  }

  fn work_size(&self) -> Option<u32> {
    self.shuffle_idx.work_size()
  }
}

#[pollster::test]
async fn test() {
  let input = [0, 1, 2, 3, 4, 5].to_vec();
  let move_target = [5, 4, 3, 2, 1, 0].to_vec();
  let expect = [5, 4, 3, 2, 1, 0].to_vec();

  input
    .shuffle_move(move_target.map(|v| (v, val(true))))
    .run_test(&expect)
    .await
}
