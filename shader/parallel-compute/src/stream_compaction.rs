use crate::*;

/// stream compaction will reduce the work size in device, so extra care is required
#[derive(Clone)]
pub struct StreamCompaction<T> {
  pub source: Box<dyn DeviceParallelComputeIO<T>>,
  pub filter: Box<dyn DeviceParallelComputeIO<bool>>,
}

impl<T> DeviceParallelCompute<Node<T>> for StreamCompaction<T>
where
  T: Std430 + ShaderSizedValueNodeType + Debug,
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

impl<T> DeviceParallelComputeIO<T> for StreamCompaction<T>
where
  T: Std430 + ShaderSizedValueNodeType + Debug,
{
  fn materialize_storage_buffer(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> DeviceMaterializeResult<T>
  where
    T: Std430 + ShaderSizedValueNodeType,
  {
    let write_target_positions = self
      .filter
      .clone()
      .map(|v| v.select(1_u32, 0))
      .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(512, 512);

    let (_, size) = PrefixSumTailAsSize {
      prefix_sum_result: write_target_positions.execute_and_expose(cx),
    }
    .compute_work_size(cx);

    let shuffle_moved = self
      .source
      .clone()
      .shuffle_move(
        write_target_positions
          .make_global_scan_exclusive::<AdditionMonoid<u32>>()
          .zip(self.filter.clone()),
      )
      .materialize_storage_buffer(cx);

    DeviceMaterializeResult {
      buffer: shuffle_moved.buffer,
      size: size.into_readonly_view().into(),
    }
  }
}

struct PrefixSumTailAsSize {
  prefix_sum_result: Box<dyn DeviceInvocationComponent<Node<u32>>>,
}

impl ShaderHashProvider for PrefixSumTailAsSize {
  shader_hash_type_id! {}
}

impl DeviceInvocationComponent<Node<u32>> for PrefixSumTailAsSize {
  fn work_size(&self) -> Option<u32> {
    self.prefix_sum_result.work_size()
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    DeviceInvocationTailAsSize(self.prefix_sum_result.build_shader(builder)).into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.prefix_sum_result.bind_input(builder)
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    self.prefix_sum_result.requested_workgroup_size()
  }
}

struct DeviceInvocationTailAsSize(Box<dyn DeviceInvocation<Node<u32>>>);

impl DeviceInvocation<Node<u32>> for DeviceInvocationTailAsSize {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
    self.0.invocation_logic(logic_global_id)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.0.end_point(), val(0), val(0)).into()
  }
}

#[pollster::test]
async fn test_stream_compaction() {
  let input = vec![1, 0, 1, 0, 1, 1, 0];
  let expect = vec![1, 1, 1, 1, 0, 0, 0];

  let mask = input.clone().map(|v| v.equals(1));

  input
    .stream_compaction(mask)
    .run_test_with_size_test(&expect, Some(Vec3::new(4, 0, 0)))
    .await
}

#[pollster::test]
async fn test_stream_compaction2() {
  let input = vec![1, 0, 1, 0, 1, 1, 0];
  let expect = vec![1, 1, 1, 1, 0, 0, 0];

  input
    .stream_compaction_self_filter(|v| v.equals(1))
    .run_test_with_size_test(&expect, Some(Vec3::new(4, 0, 0)))
    .await
}
