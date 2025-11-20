use crate::*;

pub fn stream_compaction<T>(
  source: Box<dyn DeviceInvocationComponentIO<T>>,
  filter: Box<dyn DeviceInvocationComponentIO<bool>>,
  cx: &mut DeviceParallelComputeCtx,
) -> DeviceMaterializeResult<T>
where
  T: Std430 + ShaderSizedValueNodeType + Debug,
{
  let write_target_positions = filter
    .clone()
    .map(|v| v.select(1_u32, 0))
    .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(512, 512, cx);

  let (_, size) = PrefixSumTailAsSize {
    prefix_sum_result: Box::new(write_target_positions.clone()),
  }
  .compute_work_size(cx);

  let shuffle_moved = source.clone().shuffle_move(
    write_target_positions
      .make_global_scan_exclusive::<AdditionMonoid<u32>>()
      .zip(filter.clone()),
    cx,
  );

  DeviceMaterializeResult {
    buffer: shuffle_moved.buffer,
    size: size.into_readonly_view().into(),
  }
}

#[derive(Clone)]
struct PrefixSumTailAsSize {
  prefix_sum_result: Box<dyn DeviceInvocationComponent<Node<u32>>>,
}

impl ShaderHashProvider for PrefixSumTailAsSize {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.prefix_sum_result.hash_pipeline_with_type_info(hasher);
  }
}

impl DeviceInvocationComponent<Node<u32>> for PrefixSumTailAsSize {
  fn work_size(&self) -> Option<u32> {
    self.prefix_sum_result.work_size()
  }

  fn result_size(&self) -> u32 {
    self.prefix_sum_result.result_size()
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
  gpu_test_scope(async |cx| {
    let input = vec![1, 0, 1, 0, 1, 1, 0];
    let expect = vec![1, 1, 1, 1, 0, 0, 0];

    let input = slice_into_compute(&input, cx);
    let mask = input.clone().map(|v| v.equals(1));

    input
      .stream_compaction(mask, cx)
      .run_test_with_size_test(cx, &expect, Some(Vec3::new(4, 0, 0)))
      .await
  })
  .await
}

#[pollster::test]
async fn test_stream_compaction2() {
  gpu_test_scope(async |cx| {
    let input = vec![1, 0, 1, 0, 1, 1, 0];
    let expect = vec![1, 1, 1, 1, 0, 0, 0];

    slice_into_compute(&input, cx)
      .stream_compaction_self_filter(|v| v.equals(1), cx)
      .run_test_with_size_test(cx, &expect, Some(Vec3::new(4, 0, 0)))
      .await;
  })
  .await
}
