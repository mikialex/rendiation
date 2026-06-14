use std::hash::Hash;

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
pub fn downgrade_multi_indirect_draw_count_list_pool(
  input: MIDCListPoolInput,
  cx: &mut DeviceParallelComputeCtx,
) -> Vec<(DowngradeMultiIndirectDrawCountHelper, DrawCommand)> {
  let total_capacity = input.list_info.sum_all_count_host;
  let list_count = input.list_info.host_capacity_ranges.len() as u32;

  if total_capacity == 0 {
    return Vec::new();
  }

  let is_indexed = input.command_pool.is_index();

  // segmented prefix scan over all vertex counts
  let inclusive_scan_result = ListPoolVertexCountSource {
    command_pool: input.command_pool.clone(),
    sub_list_ranges: input.list_info.sub_list_ranges.clone(),
    sum_all_count: input.list_info.sum_all_count.clone(),
    total_capacity,
  }
  .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(1024, 1024, cx)
  .materialize_storage_buffer(cx);

  let inclusive_scan_result = inclusive_scan_result.buffer;

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
  let total_padded_entries = offset;
  let output_prefix_segments_ranges =
    create_gpu_readonly_storage(output_prefix_segments_ranges_host.as_slice(), &cx.gpu);

  // Per-sub-list relative exclusive prefix sums: each sub-list gets capacity_i + 1 entries
  // followed by padding zeros to satisfy storage buffer offset alignment.
  let output_prefix: StorageBufferDataView<[u32]> = create_gpu_read_write_storage(
    ZeroedArrayByArrayLength(total_padded_entries as usize),
    &cx.gpu,
  );

  let aligned_counts: StorageBufferDataView<[u32]> = create_gpu_read_write_storage(
    ZeroedArrayByArrayLength(list_count as usize * align_u32 as usize),
    &cx.gpu,
  );

  // Combined indirect args: one per sub-list (always use DrawIndirectArgsStorage —
  // the downgraded draw is always non-indexed indirect)
  let output_indirect: StorageBufferDataView<[DrawIndirectArgsStorage]> =
    StorageBufferDataView::create_by_with_extra_usage(
      cx.gpu.device.as_ref(),
      StorageBufferInit::from(ZeroedArrayByArrayLength(list_count as usize)),
      BufferUsages::INDIRECT,
    );

  // compute indirect dispatch size from sum_all_count
  let dispatch_indirect = StorageBufferDataView::create_by_with_extra_usage(
    cx.gpu.device.as_ref(),
    StorageBufferInit::<DispatchIndirectArgsStorage>::from(StorageBufferSizedZeroed::<
      DispatchIndirectArgsStorage,
    >::default()),
    BufferUsages::INDIRECT,
  );

  cx.record_pass(|pass, device| {
    let hasher = shader_hasher_from_marker_ty!(ListPoolDowngradeDispatchSize);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);
      let sum_all = builder.bind_by(&input.list_info.sum_all_count).load();
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
      .with_bind(&input.list_info.sum_all_count)
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
      let sub_list_ranges = builder.bind_by(&input.list_info.sub_list_ranges);
      let sum_all = builder.bind_by(&input.list_info.sum_all_count).load();
      let output_prefix = builder.bind_by(&output_prefix);
      let output_indirect = builder.bind_by(&output_indirect);
      let output_prefix_segments_ranges = builder.bind_by(&output_prefix_segments_ranges);
      let aligned_counts = builder.bind_by(&aligned_counts);

      let global_idx = builder.global_invocation_id().x();
      let sub_list_count = sub_list_ranges.array_length();

      // write per-sub-list relative prefix sums
      let is_valid = global_idx.less_than(sum_all);
      if_by(is_valid, || {
        // Binary search for sub-list containing this global index
        let low = val(0u32).make_local_var();
        let high = sub_list_count.make_local_var();
        let found = val(0u32).make_local_var();

        loop_by(|cx| {
          let lo = low.load();
          let hi = high.load();
          let done = lo.greater_than(hi).or(lo.equals(hi));
          if_by(done, || cx.do_break());

          let mid = (lo + hi) / val(2u32);
          let z_mid = sub_list_ranges.index(mid).count_prefix_sum().load();

          let p_le_id = z_mid.less_than(global_idx).or(z_mid.equals(global_idx));
          if_by(p_le_id, || {
            found.store(mid);
            low.store(mid + val(1u32));
          })
          .else_by(|| {
            high.store(mid);
          });
        });

        let list_idx = found.load();
        let range = sub_list_ranges.index(list_idx).load().expand();
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
      .with_bind(&input.list_info.sub_list_ranges)
      .with_bind(&input.list_info.sum_all_count)
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
pub fn downgrade_multi_indirect_draw_count(
  draw: DrawCommand,
  cx: &mut DeviceParallelComputeCtx,
) -> (DowngradeMultiIndirectDrawCountHelper, DrawCommand) {
  if let DrawCommand::MultiIndirectCount {
    indexed,
    indirect_buffer,
    indirect_count,
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
    let draw_count = StorageBufferReadonlyDataView::try_from_raw(indirect_count).unwrap();

    // Build single-sub-list MultiRangeDispatchInfo
    let ranges_init = vec![StorageSubListRangeInfo::new(0, max_count, 0)];
    let sub_list_ranges = create_gpu_readonly_storage(ranges_init.as_slice(), &cx.gpu);

    let list_info = MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count: draw_count,
      host_capacity_ranges: vec![CapacityRange {
        capacity: max_count,
        offset: 0,
      }],
      sum_all_count_host: max_count,
    };

    let input = MIDCListPoolInput {
      command_pool: draw_commands,
      list_info,
    };

    let mut results = downgrade_multi_indirect_draw_count_list_pool(input, cx);
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
