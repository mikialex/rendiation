use rendiation_device_draw_list::*;
use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod host_driven;
pub use host_driven::*;
mod list_pool_vertex_counts;
use list_pool_vertex_counts::*;

mod mesh_sys_wrapper;
pub use mesh_sys_wrapper::*;

mod draw_helper;
pub use draw_helper::*;

only_vertex!(VertexIndexForMIDCDowngrade, u32);
only_vertex!(VertexIndexForMIDCDowngradeRelative, u32);

pub fn require_midc_downgrade(info: &GPUInfo, force_downgrade: bool) -> bool {
  if force_downgrade {
    return true;
  }

  if info.adaptor_info.backend == Backend::Dx12 {
    // https://github.com/gfx-rs/wgpu/issues/7974
    return true;
  }

  !info
    .supported_features
    .contains(Features::MULTI_DRAW_INDIRECT_COUNT)
}

pub struct MIDCListPoolInput {
  pub command_pool: StorageDrawCommands,
  pub list_info: MultiRangeDispatchInfo,
}

/// downgrade midc into single none-index indirect draw with helper access data.
///
/// the sub draw command not support instance count > 1
///
/// Process all sub-lists in a single batch: one prefix scan dispatch + one output write dispatch.
/// Returns per-sub-list downgrade results (helper + DrawCommand::Indirect).
pub fn use_downgrade_multi_indirect_draw_count_list_pool(
  input: MIDCListPoolInput,
  cx: &mut DeviceParallelComputeCtx,
) -> Vec<(DowngradeMultiIndirectDrawCountHelper, DrawCommand)> {
  let total_capacity = input.list_info.total_capacity;
  let list_count = input.list_info.host_capacity_ranges.len() as u32;

  let limits = &cx.gpu.info.supported_limits;
  let align_bytes = limits.min_storage_buffer_offset_alignment as u64;
  let align_u32 = (align_bytes / 4) as u32;

  // (offset, capacity) for each sub prefix output
  let mut output_prefix_segments_ranges_host = Vec::with_capacity(list_count as usize);
  let mut offset: u32 = 0;
  for info in &input.list_info.host_capacity_ranges {
    let required_capacity = info.capacity + 1; // + 1 for exclusive prefix sum last entry
    let padded_required_capacity = round_up(required_capacity, align_u32);
    output_prefix_segments_ranges_host.push(Vec2::new(offset, padded_required_capacity));
    offset += padded_required_capacity;
  }
  let total_padded_entries = offset as usize;

  let output_prefix_segments_ranges = cx.use_storage_buffer_array_with_host_data_queue_write_sync(
    &output_prefix_segments_ranges_host,
    "output_prefix_segments_ranges",
  );

  // Per-sub-list relative exclusive prefix sums: each sub-list gets capacity_i + 1 entries
  // followed by padding zeros to satisfy storage buffer offset alignment.
  let output_prefix = cx.use_rw_storage_buffer_array::<u32>(total_padded_entries, "output_prefix");

  let len = list_count as usize * align_u32 as usize;
  let aligned_counts = cx.use_rw_storage_buffer_array::<u32>(len, "aligned_counts");

  // Combined indirect args: one per sub-list (always use DrawIndirectArgsStorage —
  // the downgraded draw is always non-indexed indirect)
  let output_indirect = cx.use_rw_storage_buffer_array_impl::<DrawIndirectArgsStorage>(
    list_count as usize,
    "output_indirect one draw cmd",
    BufferUsages::INDIRECT,
  );
  // When buffer is reused across frames, sub-lists with zero elements retain stale
  // draw commands from the previous frame. The compute shader's is_last_in_sub
  // check can never be true for count=0 sub-lists (local_idx == count-1 wraps
  // to u32::MAX), so those entries are never overwritten. Writing zeros here
  // ensures clean state before the compute shader runs.
  cx.flush_pass();
  cx.encoder
    .clear_buffer(output_indirect.buffer.gpu(), 0, None);

  // compute indirect dispatch size from sum_all_count
  let dispatch_indirect = cx.use_rw_storage_buffer_impl(
    &DispatchIndirectArgsStorage::default(),
    "dispatch_indirect cmd",
    BufferUsages::INDIRECT,
  );

  let is_indexed = input.command_pool.is_index();

  let max_width = cx
    .gpu
    .info()
    .supported_limits
    .max_compute_invocations_per_workgroup;

  // segmented prefix scan over all vertex counts
  let inclusive_scan_result = ListPoolVertexCountSource {
    command_pool: input.command_pool.clone(),
    ranges: input.list_info.device_ranges.clone(),
    total_capacity,
  }
  .use_segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(max_width, max_width, cx)
  .use_materialize_storage_buffer(cx);

  let inclusive_scan_result = inclusive_scan_result.buffer;

  cx.record_pass(|pass, device| {
    let hasher = shader_hasher_from_marker_ty!(ListPoolDowngradeDispatchSize);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);
      let sum_all = builder
        .bind_by(&input.list_info.device_ranges.sum_all_count)
        .load();
      let output = builder.bind_by(&dispatch_indirect);

      let total_threads = sum_all + val(list_count);
      let wg_count = device_compute_dispatch_size(total_threads, val(256u32));
      output.store(
        ENode::<DispatchIndirectArgsStorage> {
          x: wg_count,
          y: val(1),
          z: val(1),
        }
        .construct(),
      );

      builder
    });

    BindingBuilder::default()
      .with_bind(&input.list_info.device_ranges.sum_all_count)
      .with_bind(&dispatch_indirect)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(1, 1, 1);
  });

  // write per-sub-list prefix sums and indirect args
  let dispatch_indirect_view = dispatch_indirect.gpu.clone();
  cx.record_pass(|pass, device| {
    let hasher = shader_hasher_from_marker_ty!(MidcHelperDataWrite).with_hash(is_indexed);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(256);
      let inclusive_scan_result = builder.bind_by(&inclusive_scan_result);
      let ranges = input.list_info.device_ranges.build_shader(&mut builder);
      let output_prefix = builder.bind_by(&output_prefix);
      let output_indirect = builder.bind_by(&output_indirect);
      let output_prefix_segments_ranges = builder.bind_by(&output_prefix_segments_ranges);
      let aligned_counts = builder.bind_by(&aligned_counts);

      let global_idx = builder.global_invocation_id().x();

      // write per-sub-list relative prefix sums
      let (list_idx, is_valid) = ranges.compute_list_index(global_idx);
      if_by(is_valid, || {
        let range = ranges.read_range_info(list_idx);
        let seg_start = range.count_prefix_sum;
        let count = range.count;

        let prefix_write_out_list_base = output_prefix_segments_ranges.index(list_idx).load().x();

        let local_idx = global_idx - seg_start;

        let seg_start_excl = seg_start.equals(0).select_branched(
          || val(0),
          || inclusive_scan_result.index(seg_start - val(1u32)).load(),
        );

        let global_excl = global_idx.equals(seg_start).select_branched(
          || seg_start_excl,
          || inclusive_scan_result.index(global_idx - val(1u32)).load(),
        );

        let seg_prefix_scan_exclusive = global_excl - seg_start_excl;
        output_prefix
          .index(prefix_write_out_list_base + local_idx)
          .store(seg_prefix_scan_exclusive);

        // The last thread in each sub-list also writes the total and indirect args
        let is_last_in_sub = local_idx.equals(count - val(1u32));
        if_by(is_last_in_sub, || {
          let total_count = seg_start.equals(0).select_branched(
            || inclusive_scan_result.index(global_idx).load(),
            || inclusive_scan_result.index(global_idx).load() - seg_start_excl,
          );
          output_prefix
            .index(prefix_write_out_list_base + local_idx + val(1u32))
            .store(total_count);

          let args = ENode::<DrawIndirectArgsStorage> {
            vertex_count: total_count,
            instance_count: val(1),
            base_vertex: val(0),
            base_instance: val(0),
          }
          .construct();
          output_indirect.index(list_idx).store(args);
          aligned_counts.index(val(align_u32) * list_idx).store(count);
        });
      });

      builder
    });

    BindingBuilder::default()
      .with_bind(&inclusive_scan_result)
      .with_fn(|b| input.list_info.device_ranges.bind_shader(b))
      .with_bind(&output_prefix)
      .with_bind(&output_indirect)
      .with_bind(&output_prefix_segments_ranges)
      .with_bind(&aligned_counts)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups_indirect_by_buffer_resource_view(&dispatch_indirect_view);
  });

  let mut results = Vec::with_capacity(list_count as usize);
  for (i, info) in input.list_info.host_capacity_ranges.iter().enumerate() {
    let count_offset = i as u64 * align_bytes;
    let count_view = aligned_counts.gpu.resource.create_view(GPUBufferViewRange {
      offset: count_offset,
      size: std::num::NonZeroU64::new(4).into(),
    });

    // create prefix sum views
    let offset = output_prefix_segments_ranges_host[i].x() as u64 * 4; // 4 bytes per u32;
    debug_assert!(offset.is_multiple_of(align_bytes));
    let prefix_view = output_prefix.gpu.resource.create_view(GPUBufferViewRange {
      offset,
      size: std::num::NonZeroU64::new((info.capacity + 1) as u64 * 4).into(),
    });
    let prefix_abstract: AbstractReadonlyStorageBuffer<[u32]> =
      StorageBufferReadonlyDataView::<[u32]>::try_from_raw(prefix_view)
        .unwrap()
        .into();

    let draw_count_abstract: AbstractReadonlyStorageBuffer<u32> =
      StorageBufferReadonlyDataView::<u32>::try_from_raw(count_view)
        .unwrap()
        .into();

    let draw_commands =
      create_command_pool_view(&input.command_pool, info.offset, info.capacity, align_bytes);

    let helper = DowngradeMultiIndirectDrawCountHelper {
      sub_draw_range_start_prefix_sum: prefix_abstract,
      draw_count: draw_count_abstract,
      draw_commands,
    };

    // Create indirect arg view for this sub-list (1 element)
    let item_size = std::mem::size_of::<DrawIndirectArgsStorage>() as u64;
    let indirect_view = output_indirect
      .gpu
      .resource
      .create_view(GPUBufferViewRange {
        offset: i as u64 * item_size,
        size: std::num::NonZeroU64::new(item_size).into(),
      });

    let cmd = DrawCommand::Indirect {
      indirect_buffer: indirect_view,
      indexed: false, // downgraded draw is always non-indexed indirect
    };

    results.push((helper, cmd));
  }

  results
}

fn create_command_pool_view(
  pool: &StorageDrawCommands,
  offset: u32,
  count: u32,
  align_check: u64,
) -> StorageDrawCommands {
  let item_size = match pool {
    StorageDrawCommands::Indexed(_) => std::mem::size_of::<DrawIndexedIndirectArgsStorage>(),
    StorageDrawCommands::NoneIndexed(_) => std::mem::size_of::<DrawIndirectArgsStorage>(),
  } as u64;

  let raw_view = pool.indirect_buffer();

  let offset = offset as u64 * item_size;
  debug_assert!(offset.is_multiple_of(align_check));

  let cmd_view = raw_view.resource.create_view(GPUBufferViewRange {
    offset,
    size: std::num::NonZeroU64::new(count as u64 * item_size).into(),
  });

  match pool {
    StorageDrawCommands::Indexed(_) => {
      let view = StorageBufferReadonlyDataView::try_from_raw(cmd_view).unwrap();
      StorageDrawCommands::Indexed(view.into())
    }
    StorageDrawCommands::NoneIndexed(_) => {
      let view = StorageBufferReadonlyDataView::try_from_raw(cmd_view).unwrap();
      StorageDrawCommands::NoneIndexed(view.into())
    }
  }
}

/// downgrade midc into single none-index indirect draw with helper access data.
///
/// the sub draw command not support instance count > 1
///
/// This is now a thin wrapper around `downgrade_multi_indirect_draw_count_list_pool`
/// for the single-sub-list case.
pub fn use_downgrade_multi_indirect_draw_count(
  draw: DrawCommand,
  cx: &mut DeviceParallelComputeCtx,
) -> (DowngradeMultiIndirectDrawCountHelper, DrawCommand) {
  if let DrawCommand::MultiIndirectCount {
    indexed,
    indirect_buffer,
    indirect_count: _,
    max_count,
  } = draw
  {
    let draw_commands = if indexed {
      StorageDrawCommands::Indexed(
        StorageBufferReadonlyDataView::try_from_raw(indirect_buffer)
          .unwrap()
          .into(),
      )
    } else {
      StorageDrawCommands::NoneIndexed(
        StorageBufferReadonlyDataView::try_from_raw(indirect_buffer)
          .unwrap()
          .into(),
      )
    };
    assert!(draw_commands.cmd_capacity_count() > 0);

    // single list
    let ranges_init = vec![StorageSubListRangeInfo::new(0, max_count, 0)];
    let device_ranges = DeviceMultiRangeDispatchInfo::new(&cx.gpu, &ranges_init);

    assert!(max_count > 0);
    let host_capacity_ranges = vec![CapacityRange {
      capacity: max_count,
      offset: 0,
    }];

    let list_info = MultiRangeDispatchInfo {
      device_ranges,
      host_capacity_ranges,
      total_capacity: max_count,
    };

    let input = MIDCListPoolInput {
      command_pool: draw_commands,
      list_info,
    };

    let mut results = use_downgrade_multi_indirect_draw_count_list_pool(input, cx);
    results.pop().unwrap()
  } else {
    panic!("expect midc draw command");
  }
}

pub(crate) fn round_up(value: u32, alignment: u32) -> u32 {
  (value + alignment - 1) / alignment * alignment
}

#[cfg(test)]
mod tests;
