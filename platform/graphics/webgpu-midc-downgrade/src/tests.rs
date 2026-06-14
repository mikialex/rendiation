use crate::*;

/// Build a non-indexed command pool with 3 sub-lists.
/// Sub-list capacities are padded to satisfy storage buffer offset alignment
/// (each DrawIndirectArgsStorage is 16 bytes; min_storage_buffer_offset_alignment
/// is typically 256 bytes, so pool indices must be multiples of 16).
///
///   Sub-list 0: vertex counts [3, 5] → total 8  (at pool indices 0..1)
///   Sub-list 1: vertex counts [2, 4, 6] → total 12  (at pool indices 16..18)
///   Sub-list 2: vertex counts [7, 1] → total 8  (at pool indices 32..33)
fn build_test_input(gpu: &GPU) -> MIDCListPoolInput {
  // Each command is 16 bytes, alignment is 256 → pool stride = 16 commands
  const CMD_ALIGN: u32 = 16;

  let real_counts = [2u32, 3, 2];
  let padded_capacities: Vec<u32> = real_counts
    .iter()
    .map(|c| round_up(*c, CMD_ALIGN))
    .collect();
  let mut padded_offsets = Vec::with_capacity(3);
  let mut running = 0u32;
  for &pc in &padded_capacities {
    padded_offsets.push(running);
    running += pc;
  }
  let total_pool_size = running;

  // Build padded command pool with real commands at their padded offsets
  let mut cmds = vec![DrawIndirectArgsStorage::new(0, 0, 0, 0); total_pool_size as usize];
  cmds[padded_offsets[0] as usize] = DrawIndirectArgsStorage::new(3, 1, 0, 0);
  cmds[padded_offsets[0] as usize + 1] = DrawIndirectArgsStorage::new(5, 1, 0, 0);
  cmds[padded_offsets[1] as usize] = DrawIndirectArgsStorage::new(2, 1, 0, 0);
  cmds[padded_offsets[1] as usize + 1] = DrawIndirectArgsStorage::new(4, 1, 0, 0);
  cmds[padded_offsets[1] as usize + 2] = DrawIndirectArgsStorage::new(6, 1, 0, 0);
  cmds[padded_offsets[2] as usize] = DrawIndirectArgsStorage::new(7, 1, 0, 0);
  cmds[padded_offsets[2] as usize + 1] = DrawIndirectArgsStorage::new(1, 1, 0, 0);

  let command_pool =
    StorageDrawCommands::NoneIndexed(create_gpu_readonly_storage(cmds.as_slice(), gpu).into());

  let mut prefix_sum = 0u32;
  let ranges_vec: Vec<StorageSubListRangeInfo> = real_counts
    .iter()
    .zip(padded_offsets.iter())
    .map(|(&count, &off)| {
      let r = StorageSubListRangeInfo::new(off, count, prefix_sum);
      prefix_sum += count;
      r
    })
    .collect();
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), gpu);
  let sum_all_count = rendiation_webgpu::create_gpu_readonly_storage(&7u32, gpu);

  let host_capacity_ranges = real_counts
    .iter()
    .zip(padded_capacities.iter())
    .zip(padded_offsets.iter())
    .map(|((_real_count, &padded_cap), &padded_off)| CapacityRange {
      capacity: padded_cap,
      offset: padded_off,
    })
    .collect();

  let list_info = MultiRangeDispatchInfo {
    sub_list_ranges,
    sum_all_count,
    host_capacity_ranges,
    sum_all_count_host: total_pool_size,
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
  // Capacities are padded to CMD_ALIGN (16) for storage buffer offset alignment
  let expected_capacities = [16u32, 16, 16];
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

  let ranges_vec = vec![StorageSubListRangeInfo::new(0, 1, 0)];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), &gpu);
  let sum_all_count = create_gpu_readonly_storage(&1u32, &gpu);

  let list_info = MultiRangeDispatchInfo {
    sub_list_ranges,
    sum_all_count,
    host_capacity_ranges: vec![CapacityRange {
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

  let ranges_vec = vec![StorageSubListRangeInfo::new(0, 3, 0)];
  let sub_list_ranges = create_gpu_readonly_storage(ranges_vec.as_slice(), &gpu);
  let sum_all_count = rendiation_webgpu::create_gpu_readonly_storage(&3u32, &gpu);

  let list_info = MultiRangeDispatchInfo {
    sub_list_ranges,
    sum_all_count,
    host_capacity_ranges: vec![CapacityRange {
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
