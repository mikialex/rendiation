use crate::{stream_compact::ListOfListsCullingPredicate, *};

/// One sub-list of the test input. The ids are the real content, the capacity is the size of
/// the pool region.
struct TestSubList {
  capacity: u32,
  ids: Vec<u32>,
}

fn sub(capacity: u32, ids: &[u32]) -> TestSubList {
  assert!(ids.len() as u32 <= capacity);
  TestSubList {
    capacity,
    ids: ids.to_vec(),
  }
}

/// Build the draw list from the sub-lists, the pool regions are placed at the given offsets.
/// The unused capacity is filled by a marker that should never appear in the output.
fn build_draw_list_with_offsets(
  gpu: &GPU,
  sub_lists: &[TestSubList],
  offsets: &[u32],
) -> DeviceDrawList {
  const UNUSED_MARKER: u32 = u32::MAX;
  let host_capacity_ranges: Vec<_> = sub_lists
    .iter()
    .zip(offsets)
    .map(|(sub_list, &offset)| CapacityRange {
      capacity: sub_list.capacity,
      offset,
    })
    .collect();

  let pool_size = host_capacity_ranges
    .iter()
    .map(|r| r.offset + r.capacity)
    .max()
    .unwrap();
  let mut pool = vec![UNUSED_MARKER; pool_size as usize];
  for (sub_list, range) in sub_lists.iter().zip(&host_capacity_ranges) {
    let start = range.offset as usize;
    pool[start..start + sub_list.ids.len()].copy_from_slice(&sub_list.ids);
  }
  let id_pool = create_gpu_readonly_storage(pool.as_slice(), gpu, "scene_model_id_pool");

  let lengths: Vec<_> = sub_lists.iter().map(|s| s.ids.len() as u32).collect();
  let ranges = prepare_gpu_sub_list_ranges(&host_capacity_ranges, &lengths);
  let device_ranges = DeviceMultiRangeDispatchInfo::new(gpu, ranges.as_slice());

  DeviceDrawList {
    id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      device_ranges,
      host_capacity_ranges,
      total_capacity: lengths.iter().sum(),
    },
  }
}

fn build_draw_list(gpu: &GPU, sub_lists: &[TestSubList]) -> DeviceDrawList {
  let capacities: Vec<_> = sub_lists
    .iter()
    .map(|s| CapacityRange {
      capacity: s.capacity,
      offset: 0,
    })
    .collect();
  let (ranges, _) = compact_capacity_ranges(&capacities);
  let offsets: Vec<_> = ranges.iter().map(|r| r.offset).collect();
  build_draw_list_with_offsets(gpu, sub_lists, &offsets)
}

/// Culler that keeps only elements with id <= 40.
#[derive(Clone)]
struct KeepLeq40Culler;
impl ShaderHashProvider for KeepLeq40Culler {
  shader_hash_type_id! {}
}
impl AbstractCullerProvider for KeepLeq40Culler {
  fn create_invocation(&self, _: &mut ShaderBindGroupBuilder) -> Box<dyn AbstractCullerInvocation> {
    Box::new(KeepLeq40CullerInvocation)
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}
struct KeepLeq40CullerInvocation;
impl AbstractCullerInvocation for KeepLeq40CullerInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    id.greater_than(val(40u32))
  }
}

fn keep_all(_: u32) -> bool {
  true
}
fn keep_leq_40(id: u32) -> bool {
  id <= 40
}

async fn read_storage_u32(
  cx: &mut DeviceParallelComputeCtx<'_>,
  buffer: &StorageBufferReadonlyDataView<[u32]>,
) -> Vec<u32> {
  cx.flush_pass();
  let fut = cx.encoder.read_buffer(&cx.gpu.device, &buffer.gpu);
  cx.submit_recorded_work_and_continue();
  <[u32]>::from_bytes_into_boxed(&fut.await.unwrap().read_raw()).into_vec()
}

async fn read_storage_sub_list_range_info(
  cx: &mut DeviceParallelComputeCtx<'_>,
  buffer: &StorageBufferReadonlyDataView<[StorageSubListRangeInfo]>,
) -> Vec<StorageSubListRangeInfo> {
  cx.flush_pass();
  let fut = cx.encoder.read_buffer(&cx.gpu.device, &buffer.gpu);
  cx.submit_recorded_work_and_continue();
  <[StorageSubListRangeInfo]>::from_bytes_into_boxed(&fut.await.unwrap().read_raw()).into_vec()
}

async fn read_storage_scalar_u32(
  cx: &mut DeviceParallelComputeCtx<'_>,
  buffer: &StorageBufferReadonlyDataView<u32>,
) -> u32 {
  cx.flush_pass();
  let fut = cx.encoder.read_buffer(&cx.gpu.device, &buffer.gpu);
  cx.submit_recorded_work_and_continue();
  <[u32]>::from_bytes_into_boxed(&fut.await.unwrap().read_raw())[0]
}

/// Check the culled list matches the cpu reference: the survivors of each sub-list keep their
/// order and are packed at the start of the sub-list's region, the regions are placed by
/// [compact_capacity_ranges], and the device offsets equal to the host offsets.
async fn check_culled_list(
  cx: &mut DeviceParallelComputeCtx<'_>,
  result: &DeviceDrawList,
  input: &[TestSubList],
  keep: impl Fn(u32) -> bool,
) {
  let capacities: Vec<_> = input
    .iter()
    .map(|s| CapacityRange {
      capacity: s.capacity,
      offset: 0,
    })
    .collect();
  let (expected_layout, _) = compact_capacity_ranges(&capacities);
  assert_eq!(result.dispatch_info.host_capacity_ranges, expected_layout);

  let survivors: Vec<Vec<u32>> = input
    .iter()
    .map(|s| s.ids.iter().copied().filter(|id| keep(*id)).collect())
    .collect();
  let lengths: Vec<_> = survivors.iter().map(|s| s.len() as u32).collect();
  let expected_ranges = prepare_gpu_sub_list_ranges(&expected_layout, &lengths);

  let ranges =
    read_storage_sub_list_range_info(cx, &result.dispatch_info.device_ranges.sub_list_ranges).await;
  assert_eq!(ranges, expected_ranges, "sub-list ranges");

  let total = read_storage_scalar_u32(cx, &result.dispatch_info.device_ranges.sum_all_count).await;
  assert_eq!(total, lengths.iter().sum::<u32>(), "total survivor count");

  let pool = read_storage_u32(cx, &result.id_pool).await;
  for (i, (range, expected)) in expected_layout.iter().zip(&survivors).enumerate() {
    let start = range.offset as usize;
    assert_eq!(
      &pool[start..start + expected.len()],
      expected.as_slice(),
      "survivors of sub-list {i}"
    );
  }
}

async fn run_culling_case(input: Vec<TestSubList>) {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_draw_list(&gpu, &input);

  cx.next_scope_index();
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));
  check_culled_list(&mut cx, &result, &input, keep_all).await;

  cx.next_scope_index();
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler));
  check_culled_list(&mut cx, &result, &input, keep_leq_40).await;
}

#[pollster::test]
async fn test_predicate_mask_noop() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let input = [sub(2, &[10, 20]), sub(3, &[30, 40, 50]), sub(2, &[60, 70])];
  let draw_list = build_draw_list(&gpu, &input);
  let predicate = ListOfListsCullingPredicate {
    draw_list,
    culler: Box::new(NoopCuller),
  };
  let mask = predicate.use_materialize_storage_buffer(&mut cx);
  let mask_data = read_storage_u32(&mut cx, &mask.buffer).await;
  assert_eq!(
    mask_data[..7],
    [1, 1, 1, 1, 1, 1, 1],
    "NoopCuller mask should be all 1s"
  );
}

#[pollster::test]
async fn test_draw_list_culling_basic() {
  run_culling_case(vec![
    sub(2, &[10, 20]),
    sub(3, &[30, 40, 50]),
    sub(2, &[60, 70]),
  ])
  .await;
}

/// mimics the output of a prior culling pass where all elements in the first sub-list were
/// culled, this triggers u32 underflow without the guard.
#[pollster::test]
async fn test_draw_list_culling_empty_first_sub_list() {
  run_culling_case(vec![sub(2, &[]), sub(3, &[30, 40]), sub(2, &[60, 70])]).await;
}

#[pollster::test]
async fn test_draw_list_culling_two_empty_first_sub_lists() {
  run_culling_case(vec![sub(2, &[]), sub(3, &[]), sub(2, &[60, 70])]).await;
}

#[pollster::test]
async fn test_draw_list_culling_empty_middle_sub_list() {
  run_culling_case(vec![sub(2, &[10, 20]), sub(3, &[]), sub(2, &[60, 70])]).await;
}

#[pollster::test]
async fn test_draw_list_culling_consecutive_empty_middle_sub_lists() {
  run_culling_case(vec![
    sub(2, &[10, 20]),
    sub(2, &[]),
    sub(2, &[]),
    sub(2, &[30, 40]),
  ])
  .await;
}

/// The sub-list count is larger than the dispatched thread count derived from the element
/// count, the metadata of every sub-list must still be written.
#[pollster::test]
async fn test_draw_list_culling_many_sub_lists_few_elements() {
  let mut input: Vec<_> = (0..300).map(|_| sub(1, &[])).collect();
  input.push(sub(2, &[30, 50]));
  run_culling_case(input).await;
}

#[pollster::test]
async fn test_draw_list_culling_many_sparse_sub_lists() {
  let input = (0..700)
    .map(|i| match i % 7 {
      0 => sub(3, &[10, 50, 20]),
      3 => sub(2, &[60]),
      _ => sub(1, &[]),
    })
    .collect();
  run_culling_case(input).await;
}

/// The input regions are not placed compactly (like the sub-lists allocated in a shared pool),
/// the output is still placed compactly by capacity.
#[pollster::test]
async fn test_draw_list_culling_none_compact_input() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let input = [sub(2, &[10, 50]), sub(4, &[30, 40, 60]), sub(3, &[70])];
  let draw_list = build_draw_list_with_offsets(&gpu, &input, &[20, 3, 11]);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler));
  check_culled_list(&mut cx, &result, &input, keep_leq_40).await;
}

/// The culled list is a valid input of another culling.
#[pollster::test]
async fn test_draw_list_culling_chained() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let input: Vec<_> = (0..400)
    .map(|i| {
      if i % 5 == 0 {
        sub(4, &[50, 10, 60, 20])
      } else {
        sub(2, &[70])
      }
    })
    .collect();
  let draw_list =
    build_draw_list_with_offsets(&gpu, &input, &(0..400).map(|i| i * 4).collect::<Vec<_>>());

  cx.next_scope_index();
  let first = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler));
  cx.next_scope_index();
  let second = first.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));
  check_culled_list(&mut cx, &second, &input, keep_leq_40).await;
}
