use crate::{stream_compact::ListOfListsCullingPredicate, *};

/// Build a DeviceDrawList with 3 sub-lists for testing:
///
/// Sub-list 0: [10, 20]  (offset=0, count=2)
/// Sub-list 1: [30, 40, 50]  (offset=2, count=3)
/// Sub-list 2: [60, 70]  (offset=5, count=2)
///
/// Total: 7 elements, 3 sub-lists
fn build_test_draw_list(gpu: &GPU) -> DeviceDrawList {
  let model_ids: Vec<u32> = vec![10, 20, 30, 40, 50, 60, 70];
  let scene_model_id_pool =
    create_gpu_readonly_storage(model_ids.as_slice(), gpu, "scene_model_id_pool");

  let ranges_vec: Vec<StorageSubListRangeInfo> = vec![
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    StorageSubListRangeInfo {
      offset: 2,
      count: 3,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 5,
      ..Zeroable::zeroed()
    },
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu, "sub_list_ranges");
  let sum_all_count = create_gpu_readonly_storage(&7u32, gpu, "sum_all_count");

  DeviceDrawList {
    id_pool: scene_model_id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count,
      host_capacity_ranges: vec![
        CapacityRange {
          capacity: 2,
          offset: 0,
        },
        CapacityRange {
          capacity: 3,
          offset: 2,
        },
        CapacityRange {
          capacity: 2,
          offset: 5,
        },
      ],
      sum_all_count_host: 7,
    },
  }
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

#[pollster::test]
async fn test_predicate_mask_noop() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list(&gpu);
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
async fn test_draw_list_culling_noop() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list(&gpu);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));

  // All 7 elements should survive.
  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(
    pool,
    vec![10, 20, 30, 40, 50, 60, 70],
    "NoopCuller: all elements should survive"
  );

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0 ranges"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 3,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 1 ranges"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 5,
      ..Zeroable::zeroed()
    },
    "sub-list 2 ranges"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 7, "total survivor count");
}

#[pollster::test]
async fn test_draw_list_culling_partial() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list(&gpu);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler {}));

  // Elements 50, 60, 70 should be culled (id > 40).
  // Sub-list 0: [10, 20] → both survive → count=2
  // Sub-list 1: [30, 40, 50] → 30, 40 survive → count=2
  // Sub-list 2: [60, 70] → both culled → count=0
  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  // The surviving elements are packed at the beginning of each sub-list's region.
  // Zeros fill the unused capacity space.
  assert_eq!(pool[0], 10);
  assert_eq!(pool[1], 20);
  assert_eq!(pool[2], 30);
  assert_eq!(pool[3], 40);
  // pool[4..7] are garbage/unused in sub-list 1's remaining capacity and sub-list 2's region
  assert_eq!(pool.len(), 7);

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0: offset=0, count=2, excl=0"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 1: offset=2, count=2, excl=2"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 0,
      count_prefix_sum: 4,
      ..Zeroable::zeroed()
    },
    "sub-list 2: offset=5, count=0, excl=4"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 4, "total survivor count should be 4");
}

/// Build a DeviceDrawList with 3 sub-lists where the first is empty.
/// This mimics the output of a prior culling pass where all elements
/// in the first sub-list were culled.
///
/// Based on the same layout as build_test_draw_list, but after culling
/// sub-list 0's elements (ids < 30). Sub-list 1 had 1 of 3 culled.
///
/// Sub-list 0: empty  (offset=0, count=0)
/// Sub-list 1: [30, 40]  (offset=2, count=2, originally 3)
/// Sub-list 2: [60, 70]  (offset=5, count=2)
///
/// Total: 4 elements, 3 sub-lists. Sub-list 0 has z=0, y=0 —
/// the exact case that triggers u32 underflow without the guard.
fn build_test_draw_list_with_empty_first_sub_list(gpu: &GPU) -> DeviceDrawList {
  // Pool layout matches original offsets: [0,1]=sub-list0, [2,3,4]=sub-list1, [5,6]=sub-list2
  // Indices 0,1,4 are unused padding from culled/missing elements
  let model_ids: Vec<u32> = vec![0, 0, 30, 40, 0, 60, 70];
  let scene_model_id_pool =
    create_gpu_readonly_storage(model_ids.as_slice(), gpu, "scene_model_id_pool");

  let ranges_vec: Vec<StorageSubListRangeInfo> = vec![
    StorageSubListRangeInfo {
      offset: 0,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 0: offset=0, count=0, prefix_sum=0
    StorageSubListRangeInfo {
      offset: 2,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 1: offset=2, count=2, prefix_sum=0
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    }, // sub-list 2: offset=5, count=2, prefix_sum=2
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu, "sub_list_ranges");
  let sum_all_count = create_gpu_readonly_storage(&4u32, gpu, "sum_all_count");

  DeviceDrawList {
    id_pool: scene_model_id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count,
      // Capacities must match the original capacities (see build_test_draw_list),
      // not the current survivor counts — offsets are derived from original capacities.
      host_capacity_ranges: vec![
        CapacityRange {
          capacity: 2,
          offset: 0,
        },
        CapacityRange {
          capacity: 3,
          offset: 2,
        },
        CapacityRange {
          capacity: 2,
          offset: 5,
        },
      ],
      sum_all_count_host: 7, // 2+3+2 = total capacity
    },
  }
}

#[pollster::test]
async fn test_draw_list_culling_empty_first_sub_list_noop() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_empty_first_sub_list(&gpu);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));

  // All 4 elements should survive: 30, 40, 60, 70
  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 7, "pool capacity should be 7 (2+3+2)");
  // Survivors packed into their sub-list regions:
  // Sub-list 0: empty → pool[0], pool[1] unused
  // Sub-list 1: [30, 40] at offset 2 → pool[2], pool[3]; pool[4] unused (capacity 3→2 survivors)
  // Sub-list 2: [60, 70] at offset 5 → pool[5], pool[6]
  assert_eq!(pool[2], 30, "sub-list 1 first survivor");
  assert_eq!(pool[3], 40, "sub-list 1 second survivor");
  assert_eq!(pool[5], 60, "sub-list 2 first survivor");
  assert_eq!(pool[6], 70, "sub-list 2 second survivor");

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0 remains empty"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 1: offset=2, 2 survivors, excl=0"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 2: offset=5, 2 survivors, excl=2"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 4, "total survivor count should be 4");
}

#[pollster::test]
async fn test_draw_list_culling_empty_first_sub_list_partial() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_empty_first_sub_list(&gpu);

  // KeepLeq40Culler: culls ids > 40 → 60, 70 are culled.
  // Survivors: 30, 40 from sub-list 1; sub-list 2 becomes empty.
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler {}));

  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 7);
  // Sub-list 1: [30, 40] at pool[2], pool[3]
  assert_eq!(pool[2], 30);
  assert_eq!(pool[3], 40);
  // Sub-list 2: both culled → pool[5], pool[6] should remain zero
  assert_eq!(pool[5], 0, "sub-list 2 first slot should be empty");
  assert_eq!(pool[6], 0, "sub-list 2 second slot should be empty");

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0 still empty"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 1: 2 survivors"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 2: 0 survivors, excl=2"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 2, "only 30 and 40 survive");
}

/// Build a DeviceDrawList with 3 sub-lists where the first TWO are empty.
/// This tests the cascading empty sub-list case: p_prev computation for sub-list 2
/// must walk back through two consecutive empty sub-lists without underflow.
///
/// Sub-list 0: empty  (offset=0, count=0)
/// Sub-list 1: empty  (offset=2, count=0)
/// Sub-list 2: [60, 70]  (offset=5, count=2)
///
/// Total: 2 elements, 3 sub-lists. Both sub-lists 0 and 1 have z=0, y=0.
fn build_test_draw_list_with_two_empty_first_sub_lists(gpu: &GPU) -> DeviceDrawList {
  // Pool: [0, 0, 0, 0, 0, 60, 70] — all slots except 5,6 are unused
  let model_ids: Vec<u32> = vec![0, 0, 0, 0, 0, 60, 70];
  let scene_model_id_pool =
    create_gpu_readonly_storage(model_ids.as_slice(), gpu, "scene_model_id_pool");

  let ranges_vec: Vec<StorageSubListRangeInfo> = vec![
    StorageSubListRangeInfo {
      offset: 0,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 0: empty
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 1: empty, z still 0 (no preceding elements contributed)
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 2: offset=5, count=2, z=0 (no preceding elements)
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu, "sub_list_ranges");
  let sum_all_count = create_gpu_readonly_storage(&2u32, gpu, "sum_all_count");

  DeviceDrawList {
    id_pool: scene_model_id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count,
      host_capacity_ranges: vec![
        CapacityRange {
          capacity: 2,
          offset: 0,
        },
        CapacityRange {
          capacity: 3,
          offset: 2,
        },
        CapacityRange {
          capacity: 2,
          offset: 5,
        },
      ],
      sum_all_count_host: 7,
    },
  }
}

#[pollster::test]
async fn test_draw_list_culling_two_empty_first_sub_lists_noop() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_two_empty_first_sub_lists(&gpu);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));

  // Only 2 elements survive: 60, 70
  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 7);
  // Sub-list 0: empty — pool[0], pool[1] unused
  // Sub-list 1: empty — pool[2..4] unused
  // Sub-list 2: [60, 70] at offset 5 → pool[5], pool[6]
  assert_eq!(pool[5], 60, "sub-list 2 first survivor");
  assert_eq!(pool[6], 70, "sub-list 2 second survivor");

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0 remains empty"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 1 remains empty"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 2: offset=5, 2 survivors, excl=0"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 2, "total survivor count should be 2");
}

#[pollster::test]
async fn test_draw_list_culling_two_empty_first_sub_lists_partial() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_two_empty_first_sub_lists(&gpu);

  // KeepLeq40Culler: culls ids > 40 → 60, 70 are both culled.
  // Result: all sub-lists become empty.
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler {}));

  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 7);
  // Sub-list 2: both culled → pool[5], pool[6] remain zero
  assert_eq!(pool[5], 0, "sub-list 2 first slot should be empty");
  assert_eq!(pool[6], 0, "sub-list 2 second slot should be empty");

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0 empty"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 1 empty"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 0,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 2: 0 survivors, excl still 0 (all preceding empty + current empty)"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 0, "no survivors at all");
}

/// Build a DeviceDrawList where the middle sub-list is empty (z > 0, y = 0).
/// This tests that the normal path (without guard) correctly handles empty
/// sub-lists not at the front: since z > 0, `z + y - 1 = z - 1` is safe.
///
/// Sub-list 0: [10, 20]  (offset=0, count=2)
/// Sub-list 1: empty  (offset=2, count=0)
/// Sub-list 2: [60, 70]  (offset=5, count=2)
///
/// Total: 4 elements, sub-list 1 has z=2, y=0.
fn build_test_draw_list_with_empty_middle_sub_list(gpu: &GPU) -> DeviceDrawList {
  // Pool: [10, 20, 0, 0, 0, 60, 70] — sub-list 1 slots (2..4) are unused
  let model_ids: Vec<u32> = vec![10, 20, 0, 0, 0, 60, 70];
  let scene_model_id_pool =
    create_gpu_readonly_storage(model_ids.as_slice(), gpu, "scene_model_id_pool");

  let ranges_vec: Vec<StorageSubListRangeInfo> = vec![
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 0: offset=0, count=2, prefix_sum=0
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    }, // sub-list 1: empty, z=2 (>0, guard won't trigger)
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    }, // sub-list 2: offset=5, count=2, prefix_sum=2
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu, "sub_list_ranges");
  let sum_all_count = create_gpu_readonly_storage(&4u32, gpu, "sum_all_count");

  DeviceDrawList {
    id_pool: scene_model_id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count,
      host_capacity_ranges: vec![
        CapacityRange {
          capacity: 2,
          offset: 0,
        },
        CapacityRange {
          capacity: 3,
          offset: 2,
        },
        CapacityRange {
          capacity: 2,
          offset: 5,
        },
      ],
      sum_all_count_host: 7,
    },
  }
}

#[pollster::test]
async fn test_draw_list_culling_empty_middle_sub_list_noop() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_empty_middle_sub_list(&gpu);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));

  // 4 survivors: 10, 20, 60, 70
  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 7);
  // Sub-list 0: [10, 20] at pool[0], pool[1]
  assert_eq!(pool[0], 10);
  assert_eq!(pool[1], 20);
  // Sub-list 1: empty — pool[2..4] unused
  // Sub-list 2: [60, 70] at pool[5], pool[6]
  assert_eq!(pool[5], 60);
  assert_eq!(pool[6], 70);

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0: 2 survivors"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 1: still empty, excl=2"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 2: 2 survivors, excl=2"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 4, "total survivor count should be 4");
}

#[pollster::test]
async fn test_draw_list_culling_empty_middle_sub_list_partial() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_empty_middle_sub_list(&gpu);

  // KeepLeq40Culler: keeps only ids <= 40 → culls 60, 70 from sub-list 2.
  // Survivors: 10, 20 from sub-list 0; sub-list 1 already empty; sub-list 2 becomes empty.
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler {}));

  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 7);
  assert_eq!(pool[0], 10);
  assert_eq!(pool[1], 20);
  // Sub-list 2: both culled → pool[5], pool[6] remain zero
  assert_eq!(pool[5], 0);
  assert_eq!(pool[6], 0);

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0: 2 survivors"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 1: still empty"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 5,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 2: now also empty, excl=2"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 2, "only 10 and 20 survive");
}

/// Build a DeviceDrawList with 4 sub-lists where two consecutive middle ones are empty.
/// This tests that the normal path (z > 0, no guard triggered) handles cascading
/// empty sub-lists that are not at the front.
///
/// Sub-list 0: [10, 20]  (offset=0, count=2)
/// Sub-list 1: empty  (offset=2, count=0)
/// Sub-list 2: empty  (offset=4, count=0)
/// Sub-list 3: [30, 40]  (offset=6, count=2)
///
/// Total: 4 elements. Sub-lists 1, 2 both have z=2, y=0.
fn build_test_draw_list_with_consecutive_empty_middle_sub_lists(gpu: &GPU) -> DeviceDrawList {
  // Pool: [10, 20, _, _, _, _, 30, 40] (indices 2..5 unused)
  let model_ids: Vec<u32> = vec![10, 20, 0, 0, 0, 0, 30, 40];
  let scene_model_id_pool =
    create_gpu_readonly_storage(model_ids.as_slice(), gpu, "scene_model_id_pool");

  let ranges_vec: Vec<StorageSubListRangeInfo> = vec![
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    }, // sub-list 0: offset=0, count=2, z=0
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    }, // sub-list 1: empty, z=2
    StorageSubListRangeInfo {
      offset: 4,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    }, // sub-list 2: empty, z=2 (unchanged, no contribution)
    StorageSubListRangeInfo {
      offset: 6,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    }, // sub-list 3: offset=6, count=2, z=2
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu, "sub_list_ranges");
  let sum_all_count = create_gpu_readonly_storage(&4u32, gpu, "sum_all_count");

  DeviceDrawList {
    id_pool: scene_model_id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count,
      host_capacity_ranges: vec![
        CapacityRange {
          capacity: 2,
          offset: 0,
        },
        CapacityRange {
          capacity: 2,
          offset: 2,
        },
        CapacityRange {
          capacity: 2,
          offset: 4,
        },
        CapacityRange {
          capacity: 2,
          offset: 6,
        },
      ],
      sum_all_count_host: 8,
    },
  }
}

#[pollster::test]
async fn test_draw_list_culling_consecutive_empty_middle_sub_lists_noop() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_consecutive_empty_middle_sub_lists(&gpu);
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(NoopCuller));

  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 8);
  // Sub-list 0: [10, 20] at pool[0], pool[1]
  assert_eq!(pool[0], 10);
  assert_eq!(pool[1], 20);
  // Sub-lists 1, 2: empty — pool[2..5] unused
  // Sub-list 3: [30, 40] at pool[6], pool[7]
  assert_eq!(pool[6], 30);
  assert_eq!(pool[7], 40);

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 4);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0: 2 survivors"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 1: empty, excl=2"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 4,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 2: empty, excl=2"
  );
  assert_eq!(
    ranges[3],
    StorageSubListRangeInfo {
      offset: 6,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 3: 2 survivors, excl=2"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 4, "total survivor count should be 4");
}

#[pollster::test]
async fn test_draw_list_culling_consecutive_empty_middle_sub_lists_partial() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let draw_list = build_test_draw_list_with_consecutive_empty_middle_sub_lists(&gpu);

  // KeepLeq40Culler: keeps ids <= 40 → all 4 elements survive (10, 20, 30, 40 ≤ 40).
  let result = draw_list.use_culled_list_and_do_culling(&mut cx, Box::new(KeepLeq40Culler {}));

  let pool = read_storage_u32(&mut cx, &result.id_pool).await;
  assert_eq!(pool.len(), 8);
  assert_eq!(pool[0], 10);
  assert_eq!(pool[1], 20);
  assert_eq!(pool[6], 30);
  assert_eq!(pool[7], 40);

  let ranges =
    read_storage_sub_list_range_info(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 4);
  assert_eq!(
    ranges[0],
    StorageSubListRangeInfo {
      offset: 0,
      count: 2,
      count_prefix_sum: 0,
      ..Zeroable::zeroed()
    },
    "sub-list 0: 2 survivors"
  );
  assert_eq!(
    ranges[1],
    StorageSubListRangeInfo {
      offset: 2,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 1: empty"
  );
  assert_eq!(
    ranges[2],
    StorageSubListRangeInfo {
      offset: 4,
      count: 0,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 2: empty"
  );
  assert_eq!(
    ranges[3],
    StorageSubListRangeInfo {
      offset: 6,
      count: 2,
      count_prefix_sum: 2,
      ..Zeroable::zeroed()
    },
    "sub-list 3: 2 survivors, excl=2"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 4, "all 4 survive (all ids ≤ 40)");
}
