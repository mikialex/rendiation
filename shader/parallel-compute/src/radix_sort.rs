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

  fn work_size(&self) -> u32 {
    self.is_one.work_size()
  }
}
impl DeviceParallelComputeIO<u32> for RadixShuffleMove {}

struct RadixShuffleMoveCompute {
  ones_before: StorageBufferReadOnlyDataView<[u32]>,
  is_one: Box<dyn DeviceInvocationComponent<Node<bool>>>,
}

impl ShaderHashProvider for RadixShuffleMoveCompute {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.is_one.hash_pipeline_with_type_info(hasher)
  }
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
    let invocation_size = is_one.invocation_size();
    let r = builder.entry_by(|cx| {
      let ones_before_arr = cx.bind_by(&self.ones_before);

      let zip = ones_before_arr.zip(is_one);

      let ((ones_before, is_one), valid) = zip.invocation_logic(cx.global_invocation_id());

      let r = is_one.select_branched(
        || {
          let input_size = ones_before_arr.array_length() - val(1);
          let last_ones_before_arr = input_size;
          let ones_in_total = ones_before_arr.index(last_ones_before_arr).load();
          input_size - ones_in_total + ones_before
        },
        || cx.global_invocation_id().x() - ones_before,
      );

      (r, valid)
    });

    AdhocInvocationResult(invocation_size, r.0, r.1).into_boxed()
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.is_one.bind_input(builder);
    builder.bind(&self.ones_before);
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
    .single_run_test(&expect)
    .await
}
