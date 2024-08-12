use crate::*;

pub trait DeviceRadixSortKeyLogic {
  const MAX_BITS: u32;
  type Data: ShaderSizedValueNodeType;
  fn is_one(value: Node<Self::Data>, bit_position: Node<u32>) -> Node<bool>;
}

pub struct IntBitOrderRadixSortLogic<T>(PhantomData<T>);

impl DeviceRadixSortKeyLogic for IntBitOrderRadixSortLogic<u32> {
  const MAX_BITS: u32 = u32::BITS;
  type Data = u32;
  fn is_one(value: Node<Self::Data>, bit_position: Node<u32>) -> Node<bool> {
    (value & (val(1) << bit_position)).not_equals(val(0))
  }
}

struct IterIndexHasher(u32);

impl ShaderHashProvider for IterIndexHasher {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash(hasher)
  }
  shader_hash_type_id! {}
}

// todo, impl memory coalesced version for better performance
pub fn device_radix_sort_naive<T, S>(
  input: impl DeviceParallelComputeIO<T> + 'static,
  per_pass_first_stage_workgroup_size: u32,
  per_pass_second_stage_workgroup_size: u32,
) -> Box<dyn DeviceParallelComputeIO<T>>
where
  S: DeviceRadixSortKeyLogic<Data = T>,
  T: ShaderSizedValueNodeType + Std430 + Debug,
{
  let mut result: Box<dyn DeviceParallelComputeIO<T>> = Box::new(input);
  for iter in 0..S::MAX_BITS {
    let iter_input = result.clone();

    let is_one = iter_input.clone().map_with_id_provided(
      move |data| S::is_one(data, val(iter)),
      IterIndexHasher(iter),
    );

    let ones_before = is_one
      .clone()
      .map(move |is_one| is_one.select(val(1), val(0)))
      .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(
        per_pass_first_stage_workgroup_size,
        per_pass_second_stage_workgroup_size,
      )
      .make_global_scan_exclusive::<AdditionMonoid<u32>>();

    let shuffle_idx = RadixShuffleMove {
      ones_before: Box::new(ones_before),
      is_one: Box::new(is_one),
    };

    let r = iter_input
      .shuffle_move(shuffle_idx.map(|v| (v, val(true))))
      .into_forker();

    result = Box::new(r)
  }
  result
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
struct RadixShuffleMove {
  ones_before: Box<dyn DeviceParallelComputeIO<u32>>,
  is_one: Box<dyn DeviceParallelComputeIO<bool>>,
}

impl DeviceParallelCompute<Node<u32>> for RadixShuffleMove {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    Box::new(RadixShuffleMoveCompute {
      ones_before: self.ones_before.materialize_storage_buffer(cx),
      is_one: self.is_one.execute_and_expose(cx),
    })
  }

  fn result_size(&self) -> u32 {
    self.is_one.result_size()
  }
}
impl DeviceParallelComputeIO<u32> for RadixShuffleMove {}

struct RadixShuffleMoveCompute {
  ones_before: DeviceMaterializeResult<u32>,
  is_one: Box<dyn DeviceInvocationComponent<Node<bool>>>,
}

impl ShaderHashProvider for RadixShuffleMoveCompute {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.is_one.hash_pipeline_with_type_info(hasher)
  }
  shader_hash_type_id! {}
}
impl DeviceInvocationComponent<Node<u32>> for RadixShuffleMoveCompute {
  fn requested_workgroup_size(&self) -> Option<u32> {
    self.is_one.requested_workgroup_size()
  }
  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let is_one = self.is_one.build_shader(builder);
    let ones_before = self.ones_before.build_shader(builder);

    is_one
      .zip(ones_before)
      .adhoc_invoke_with_self_size(|zip, id| {
        let ((is_one, ones_before), valid) = zip.invocation_logic(id);
        let ones_in_total = zip.1.end_point();
        let input_size = zip.1.invocation_size().x();

        let r = is_one.select(
          input_size - ones_in_total + ones_before,
          id.x() - ones_before,
        );
        (r, valid)
      })
      .into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.is_one.bind_input(builder);
    self.ones_before.bind_input(builder);
  }

  fn work_size(&self) -> Option<u32> {
    self.is_one.work_size()
  }
}

pub struct U32RadixSort;
impl DeviceRadixSortKeyLogic for U32RadixSort {
  const MAX_BITS: u32 = 32;

  type Data = u32;

  fn is_one(value: Node<Self::Data>, bit_position: Node<u32>) -> Node<bool> {
    (value & (val(1) << bit_position)).not_equals(val(0))
  }
}

#[pollster::test]
async fn test() {
  let input = [3, 1, 4, 6, 5, 2].to_vec();
  let expect = [1, 2, 3, 4, 5, 6].to_vec();

  input
    .device_radix_sort_naive::<U32RadixSort>(64, 64)
    .run_test(&expect)
    .await
}
