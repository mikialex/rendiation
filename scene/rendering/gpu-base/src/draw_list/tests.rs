use crate::*;

/// Build a DeviceDrawList with 3 sub-lists for testing:
///
/// Sub-list 0: [10, 20]  (offset=0, count=2)
/// Sub-list 1: [30, 40, 50]  (offset=2, count=3)
/// Sub-list 2: [60, 70]  (offset=5, count=2)
///
/// Total: 7 elements, 3 sub-lists
fn build_test_draw_list(gpu: &GPU) -> DeviceDrawList {
  let model_ids: Vec<u32> = vec![10, 20, 30, 40, 50, 60, 70];
  let scene_model_id_pool = create_gpu_readonly_storage(model_ids.as_slice(), gpu);

  let ranges_vec: Vec<Vec4<u32>> = vec![
    Vec4::new(0, 2, 0, 0), // sub-list 0: offset=0, count=2, prefix_sum=0
    Vec4::new(2, 3, 2, 0), // sub-list 1: offset=2, count=3, prefix_sum=2
    Vec4::new(5, 2, 5, 0), // sub-list 2: offset=5, count=2, prefix_sum=5
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu);
  let sum_all_count = create_gpu_readonly_storage(&7u32, gpu);

  use database::RawEntityHandle;
  let make_handle = |i| {
    let raw = RawEntityHandle::create_only_for_testing(i);
    unsafe { EntityHandle::<SceneModelEntity>::from_raw(raw) }
  };

  DeviceDrawList {
    scene_model_id_pool,
    dispatch_info: MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count,
      sub_list_infos: vec![
        SubListHostInfo {
          impl_select_id: make_handle(0),
          capacity: 2,
          offset: 0,
        },
        SubListHostInfo {
          impl_select_id: make_handle(1),
          capacity: 3,
          offset: 2,
        },
        SubListHostInfo {
          impl_select_id: make_handle(2),
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

async fn read_storage_vec4_u32(
  cx: &mut DeviceParallelComputeCtx<'_>,
  buffer: &StorageBufferReadonlyDataView<[Vec4<u32>]>,
) -> Vec<Vec4<u32>> {
  cx.flush_pass();
  let fut = cx.encoder.read_buffer(&cx.gpu.device, &buffer.gpu);
  cx.submit_recorded_work_and_continue();
  <[Vec4<u32>]>::from_bytes_into_boxed(&fut.await.unwrap().read_raw()).into_vec()
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
  let mask = predicate.materialize_storage_buffer(&mut cx);
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
  let pool = read_storage_u32(&mut cx, &result.scene_model_id_pool).await;
  assert_eq!(
    pool,
    vec![10, 20, 30, 40, 50, 60, 70],
    "NoopCuller: all elements should survive"
  );

  let ranges = read_storage_vec4_u32(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(ranges[0], Vec4::new(0, 2, 0, 0), "sub-list 0 ranges");
  assert_eq!(ranges[1], Vec4::new(2, 3, 2, 0), "sub-list 1 ranges");
  assert_eq!(ranges[2], Vec4::new(5, 2, 5, 0), "sub-list 2 ranges");

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
  let pool = read_storage_u32(&mut cx, &result.scene_model_id_pool).await;
  // The surviving elements are packed at the beginning of each sub-list's region.
  // Zeros fill the unused capacity space.
  assert_eq!(pool[0], 10);
  assert_eq!(pool[1], 20);
  assert_eq!(pool[2], 30);
  assert_eq!(pool[3], 40);
  // pool[4..7] are garbage/unused in sub-list 1's remaining capacity and sub-list 2's region
  assert_eq!(pool.len(), 7);

  let ranges = read_storage_vec4_u32(&mut cx, &result.dispatch_info.sub_list_ranges).await;
  assert_eq!(ranges.len(), 3);
  assert_eq!(
    ranges[0],
    Vec4::new(0, 2, 0, 0),
    "sub-list 0: offset=0, count=2, excl=0"
  );
  assert_eq!(
    ranges[1],
    Vec4::new(2, 2, 2, 0),
    "sub-list 1: offset=2, count=2, excl=2"
  );
  assert_eq!(
    ranges[2],
    Vec4::new(5, 0, 4, 0),
    "sub-list 2: offset=5, count=0, excl=4"
  );

  let total = read_storage_scalar_u32(&mut cx, &result.dispatch_info.sum_all_count).await;
  assert_eq!(total, 4, "total survivor count should be 4");
}
