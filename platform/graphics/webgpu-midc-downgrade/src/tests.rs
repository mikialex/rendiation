use crate::*;

/// Build a non-indexed command pool with 3 sub-lists:
///   Sub-list 0: vertex counts [3, 5] → total 8
///   Sub-list 1: vertex counts [2, 4, 6] → total 12
///   Sub-list 2: vertex counts [7, 1] → total 8
fn build_test_input(gpu: &GPU) -> MIDCListPoolInput {
  let cmds: Vec<DrawIndirectArgsStorage> = vec![
    DrawIndirectArgsStorage::new(3, 1, 0, 0),
    DrawIndirectArgsStorage::new(5, 1, 0, 0),
    DrawIndirectArgsStorage::new(2, 1, 0, 0),
    DrawIndirectArgsStorage::new(4, 1, 0, 0),
    DrawIndirectArgsStorage::new(6, 1, 0, 0),
    DrawIndirectArgsStorage::new(7, 1, 0, 0),
    DrawIndirectArgsStorage::new(1, 1, 0, 0),
  ];

  let command_pool =
    StorageDrawCommands::NoneIndexed(create_gpu_readonly_storage(cmds.as_slice(), gpu).into());

  let ranges_vec: Vec<Vec4<u32>> = vec![
    Vec4::new(0, 2, 0, 0), // sub-list 0: offset=0, count=2, prefix_sum=0
    Vec4::new(2, 3, 2, 0), // sub-list 1: offset=2, count=3, prefix_sum=2
    Vec4::new(5, 2, 5, 0), // sub-list 2: offset=5, count=2, prefix_sum=5
  ];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu);
  let sum_all_count = rendiation_webgpu::create_gpu_readonly_storage(&7u32, gpu);

  let list_info = MultiRangeDispatchInfo {
    sub_list_ranges,
    sum_all_count,
    sub_list_infos: vec![
      SubListHostInfo {
        capacity: 2,
        offset: 0,
      },
      SubListHostInfo {
        capacity: 3,
        offset: 2,
      },
      SubListHostInfo {
        capacity: 2,
        offset: 5,
      },
    ],
    sum_all_count_host: 7,
  };

  MIDCListPoolInput {
    command_pool,
    list_info,
  }
}

#[pollster::test]
async fn test_downgrade_list_pool_basic() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  let input = build_test_input(&gpu);
  let results = downgrade_multi_indirect_draw_count_list_pool(input, &mut cx);

  assert_eq!(results.len(), 3, "should produce 3 sub-list results");

  let expected_totals = [8u32, 12, 8];
  let expected_capacities = [2u32, 3, 2];
  for (i, (helper, cmd)) in results.iter().enumerate() {
    // Verify draw commands structure
    assert!(!helper.draw_commands.is_index());
    assert_eq!(
      helper.draw_commands.cmd_capacity_count(),
      expected_capacities[i],
      "sub-list {i}: wrong capacity"
    );

    // Verify indirect command
    let DrawCommand::Indirect {
      indirect_buffer,
      indexed,
    } = cmd
    else {
      panic!("expected Indirect draw command");
    };
    assert!(!indexed);

    // Read back the indirect buffer and verify vertex count
    cx.flush_pass();
    let fut = cx.encoder.read_buffer(&cx.gpu.device, indirect_buffer);
    cx.submit_recorded_work_and_continue();
    let data = fut.await.unwrap();
    let raw = data.read_raw().to_vec();
    let args: &DrawIndirectArgsStorage =
      bytemuck::from_bytes(&raw[..std::mem::size_of::<DrawIndirectArgsStorage>()]);
    assert_eq!(
      args.vertex_count, expected_totals[i],
      "sub-list {i}: expected total vertex count {}",
      expected_totals[i]
    );
    assert_eq!(args.instance_count, 1);
  }
}

#[pollster::test]
async fn test_downgrade_list_pool_zero_capacity() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  // sum_all_count_host=0 → early return before any GPU work
  let cmds = vec![DrawIndirectArgsStorage::new(1, 1, 0, 0)];
  let command_pool =
    StorageDrawCommands::NoneIndexed(create_gpu_readonly_storage(cmds.as_slice(), &gpu).into());

  let ranges_vec = vec![Vec4::new(0, 1, 0, 0)];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), &gpu);
  let sum_all_count = create_gpu_readonly_storage(&1u32, &gpu);

  let list_info = MultiRangeDispatchInfo {
    sub_list_ranges,
    sum_all_count,
    sub_list_infos: vec![SubListHostInfo {
      capacity: 1,
      offset: 0,
    }],
    sum_all_count_host: 0,
  };

  let input = MIDCListPoolInput {
    command_pool,
    list_info,
  };

  let results = downgrade_multi_indirect_draw_count_list_pool(input, &mut cx);
  assert!(
    results.is_empty(),
    "zero capacity should produce empty results"
  );
}

#[pollster::test]
async fn test_downgrade_list_pool_single_sub_list() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut encoder = gpu.create_encoder();
  let mut memory = Default::default();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder, &mut memory);

  // Single sub-list with 3 draw commands: vertex counts [3, 5, 7] → total 15
  let cmds: Vec<DrawIndirectArgsStorage> = vec![
    DrawIndirectArgsStorage::new(3, 1, 0, 0),
    DrawIndirectArgsStorage::new(5, 1, 0, 0),
    DrawIndirectArgsStorage::new(7, 1, 0, 0),
  ];
  let command_pool =
    StorageDrawCommands::NoneIndexed(create_gpu_readonly_storage(cmds.as_slice(), &gpu).into());

  let ranges_vec = vec![Vec4::new(0, 3, 0, 0)];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), &gpu);
  let sum_all_count = rendiation_webgpu::create_gpu_readonly_storage(&3u32, &gpu);

  let list_info = MultiRangeDispatchInfo {
    sub_list_ranges,
    sum_all_count,
    sub_list_infos: vec![SubListHostInfo {
      capacity: 3,
      offset: 0,
    }],
    sum_all_count_host: 3,
  };

  let input = MIDCListPoolInput {
    command_pool,
    list_info,
  };

  let results = downgrade_multi_indirect_draw_count_list_pool(input, &mut cx);
  assert_eq!(results.len(), 1, "single sub-list");

  let (_helper, cmd) = &results[0];
  let DrawCommand::Indirect {
    indirect_buffer,
    indexed,
  } = cmd
  else {
    panic!("expected Indirect");
  };
  assert!(!indexed);

  cx.flush_pass();
  let fut = cx.encoder.read_buffer(&cx.gpu.device, indirect_buffer);
  cx.submit_recorded_work_and_continue();
  let data = fut.await.unwrap();
  let raw = data.read_raw().to_vec();
  let args: &DrawIndirectArgsStorage =
    bytemuck::from_bytes(&raw[..std::mem::size_of::<DrawIndirectArgsStorage>()]);
  assert_eq!(args.vertex_count, 15);
  assert_eq!(args.instance_count, 1);
}
