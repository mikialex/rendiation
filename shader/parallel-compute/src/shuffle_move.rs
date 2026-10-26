use crate::*;

#[derive_where(Clone)]
pub struct ShuffleWrite<T: Std430> {
  pub input: Box<dyn ComputeComponent<(Node<T>, Node<u32>, Node<bool>)>>,
  /// shuffle access require reading any position, so we need fully materialized result here
  pub output: StorageBufferDataView<[T]>,
}

impl<T: Std430> ShaderHashProvider for ShuffleWrite<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.input.hash_pipeline_with_type_info(hasher);
  }
  shader_hash_type_id! {}
}

impl<T> ComputeComponent<Node<T>> for ShuffleWrite<T>
where
  T: Std430 + ShaderSizedValueNodeType,
{
  fn result_size(&self) -> u32 {
    self.input.result_size()
  }
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

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<T>>> {
    Box::new(self.clone())
  }
}

#[derive_where(Clone)]
pub struct ShuffleAccess<T: Std430> {
  /// shuffle access require reading any position, so we need fully materialized result here
  pub source: StorageBufferReadonlyDataView<[T]>,
  pub shuffle_idx: Box<dyn ComputeComponent<Node<u32>>>,
}

impl<T: Std430> ShaderHashProvider for ShuffleAccess<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.shuffle_idx.hash_pipeline_with_type_info(hasher);
  }
  shader_hash_type_id! {}
}

impl<T> ComputeComponent<Node<T>> for ShuffleAccess<T>
where
  T: Std430 + ShaderScalarOrVec,
{
  fn result_size(&self) -> u32 {
    self.shuffle_idx.result_size()
  }

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

        let valid = valid.and(read_idx.less_than(input.array_length()));
        let r = valid.select_branched(|| input.index(read_idx).load(), zeroed_val);

        (r, valid)
      })
      .into_boxed()
  }

  /// the bind order must match the build_shader
  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.source);
    self.shuffle_idx.bind_input(builder);
  }

  fn work_size(&self) -> Option<u32> {
    self.shuffle_idx.work_size()
  }

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<T>>> {
    Box::new(self.clone())
  }
}

#[pollster::test]
async fn test() {
  gpu_cx!(cx);
  let input = [0, 1, 2, 3, 4, 5].to_vec();
  let move_target = [5, 4, 3, 2, 1, 0].to_vec();
  let expect = [5, 4, 3, 2, 1, 0].to_vec();

  let input = slice_into_compute(&input, cx);
  let move_target = slice_into_compute(&move_target, cx);

  input
    .shuffle_move(move_target.map(|v| (v, val(true))), cx)
    .run_test(cx, &expect)
    .await
}

#[pollster::test]
async fn test_shuffle_access() {
  gpu_cx!(cx);
  let source = create_gpu_readonly_storage(&[10_u32, 20, 30, 40][..], &cx.gpu, "source");
  let shuffle_idx = slice_into_compute(&[3_u32, 2, 1, 0], cx);
  let expect = [40, 30, 20, 10].to_vec();

  ShuffleAccess {
    source,
    shuffle_idx: Box::new(shuffle_idx),
  }
  .map(|v| v)
  .run_test(cx, &expect)
  .await
}
