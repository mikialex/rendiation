use std::hash::Hash;

use rendiation_device_draw_list::*;
use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod host_driven;
pub use host_driven::*;

only_vertex!(VertexIndexForMIDCDowngrade, u32);
only_vertex!(VertexIndexForMIDCDowngradeRelative, u32);

fn round_up(value: u32, alignment: u32) -> u32 {
  (value + alignment - 1) / alignment * alignment
}

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

/// Process all sub-lists in a single batch: one prefix scan dispatch + one output write dispatch.
/// Returns per-sub-list downgrade results (helper + DrawCommand::Indirect).
pub fn downgrade_multi_indirect_draw_count_list_pool(
  input: MIDCListPoolInput,
  cx: &mut DeviceParallelComputeCtx,
) -> Vec<(DowngradeMultiIndirectDrawCountHelper, DrawCommand)> {
  let total_capacity = input.list_info.sum_all_count_host;
  let num_sub_lists = input.list_info.sub_list_infos.len() as u32;

  if total_capacity == 0 {
    return Vec::new();
  }

  let is_indexed = input.command_pool.is_index();

  // ---- Dispatch 1: segmented prefix scan over all vertex counts ----
  let source = ListPoolVertexCountSource {
    command_pool: input.command_pool.clone(),
    sub_list_ranges: input.list_info.sub_list_ranges.clone(),
    sum_all_count: input.list_info.sum_all_count.clone(),
    total_capacity,
  };

  let scan_result = source
    .segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(1024, 1024, cx)
    .make_global_scan_exclusive::<AdditionMonoid<u32>>()
    .materialize_storage_buffer(cx);

  let global_excl = scan_result.buffer;

  // ---- Compute alignment and padded strides ----
  let limits = &cx.gpu.info.supported_limits;
  let align_bytes = limits.min_storage_buffer_offset_alignment as u64;
  let stride_u32 = (align_bytes / 4) as u32;

  let mut padded_stride_info_vec = Vec::with_capacity(num_sub_lists as usize + 1);
  padded_stride_info_vec.push(stride_u32); // index 0 = count stride
  let mut total_padded_entries: u32 = 0;
  for info in &input.list_info.sub_list_infos {
    let padded = round_up(info.capacity + 1, stride_u32);
    padded_stride_info_vec.push(padded);
    total_padded_entries += padded;
  }
  let padded_stride_info = create_gpu_readonly_storage(padded_stride_info_vec.as_slice(), &cx.gpu);

  // ---- Allocate flat output buffers ----
  // Per-sub-list relative exclusive prefix sums: each sub-list gets capacity_i + 1 entries
  // followed by padding zeros to satisfy storage buffer offset alignment.
  let per_sub_prefix_flat: StorageBufferDataView<[u32]> = create_gpu_read_write_storage(
    ZeroedArrayByArrayLength(total_padded_entries as usize),
    &cx.gpu,
  );

  // Aligned per-sub-list draw counts: each count occupies one u32 at a stride_u32-aligned offset.
  let aligned_counts: StorageBufferDataView<[u32]> = create_gpu_read_write_storage(
    ZeroedArrayByArrayLength(num_sub_lists as usize * stride_u32 as usize),
    &cx.gpu,
  );

  // Combined indirect args: one per sub-list (always use DrawIndirectArgsStorage —
  // the downgraded draw is always non-indexed indirect)
  let per_sub_indirect_flat: StorageBufferDataView<[DrawIndirectArgsStorage]> =
    StorageBufferDataView::create_by_with_extra_usage(
      cx.gpu.device.as_ref(),
      StorageBufferInit::from(ZeroedArrayByArrayLength(num_sub_lists as usize)),
      BufferUsages::INDIRECT,
    );

  // ---- Dispatch 2a: compute indirect dispatch size from sum_all_count ----
  let dispatch_indirect = StorageBufferDataView::create_by_with_extra_usage(
    cx.gpu.device.as_ref(),
    StorageBufferInit::<DispatchIndirectArgsStorage>::from(StorageBufferSizedZeroed::<
      DispatchIndirectArgsStorage,
    >::default()),
    BufferUsages::INDIRECT,
  );

  cx.record_pass(|pass, device| {
    let mut hasher = PipelineHasher::default();
    std::hash::Hash::hash(&"list_pool_downgrade_dispatch_size", &mut hasher);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(1);
      let sum_all = builder.bind_by(&input.list_info.sum_all_count).load();
      let output = builder.bind_by(&dispatch_indirect);

      let total_threads = sum_all + val(num_sub_lists);
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

  // ---- Dispatch 2b: write per-sub-list prefix sums and indirect args ----
  let dispatch_indirect_view = dispatch_indirect.gpu.clone();
  cx.record_pass(|pass, device| {
    use std::hash::Hash;
    let mut hasher = PipelineHasher::default();
    is_indexed.hash(&mut hasher);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
      builder.config_work_group_size(256);
      let global_excl = builder.bind_by(&global_excl);
      let sub_list_ranges = builder.bind_by(&input.list_info.sub_list_ranges);
      let sum_all = builder.bind_by(&input.list_info.sum_all_count).load();
      let output_prefix = builder.bind_by(&per_sub_prefix_flat);
      let output_indirect = builder.bind_by(&per_sub_indirect_flat);
      let padded_stride_info = builder.bind_by(&padded_stride_info);
      let aligned_counts = builder.bind_by(&aligned_counts);

      let dispatch_thread_id = builder.global_invocation_id().x();
      let sub_list_count = sub_list_ranges.array_length();

      // ---- Phase A: write per-sub-list relative prefix sums ----
      let in_prefix_range = dispatch_thread_id.less_than(sum_all);
      if_by(in_prefix_range, || {
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
          let z_mid = sub_list_ranges.index(mid).load().z();

          let p_le_id = z_mid
            .less_than(dispatch_thread_id)
            .or(z_mid.equals(dispatch_thread_id));
          if_by(p_le_id, || {
            found.store(mid);
            low.store(mid + val(1u32));
          })
          .else_by(|| {
            high.store(mid);
          });
        });

        let list_idx = found.load();
        let range = sub_list_ranges.index(list_idx).load();
        let seg_start = range.z();
        let capacity_i = range.y();

        // Compute prefix base offset: sum of (capacity_j + 1) for j < list_idx
        let prefix_base = val(0u32).make_local_var();
        let counter = val(0u32).make_local_var();
        loop_by(|cx| {
          let c = counter.load();
          let done = c.greater_equal_than(list_idx);
          if_by(done, || cx.do_break());
          prefix_base.store(prefix_base.load() + padded_stride_info.index(c + val(1u32)).load());
          counter.store(c + val(1u32));
        });

        let local_idx = dispatch_thread_id - seg_start;

        // Write relative exclusive prefix: global_excl[global_id] - global_excl[seg_start]
        let seg_start_excl = seg_start
          .equals(val(0u32))
          .select_branched(|| val(0u32), || global_excl.index(seg_start).load());
        let value = global_excl.index(dispatch_thread_id).load() - seg_start_excl;
        output_prefix
          .index(prefix_base.load() + local_idx)
          .store(value);

        // The last thread in each sub-list also writes the total entry (at index capacity_i)
        let is_last_in_sub = local_idx.equals(capacity_i - val(1u32));
        if_by(is_last_in_sub, || {
          let total_value =
            global_excl.index(dispatch_thread_id + val(1u32)).load() - seg_start_excl;
          output_prefix
            .index(prefix_base.load() + local_idx + val(1u32))
            .store(total_value);
        });
      });

      // ---- Phase B: write per-sub-list combined indirect args ----
      let phase_b_id = dispatch_thread_id - sum_all;
      let in_indirect_range = phase_b_id.less_than(sub_list_count);
      if_by(in_indirect_range, || {
        let i = phase_b_id;
        let range = sub_list_ranges.index(i).load();
        let capacity_i = range.y();
        // Store the per-sub-list draw count into the aligned counts buffer
        let count_stride = padded_stride_info.index(val(0u32)).load();
        aligned_counts.index(i * count_stride).store(capacity_i);
        let has_draws = capacity_i.greater_than(val(0u32));
        if_by(has_draws, || {
          let seg_start = range.z();
          let seg_start_excl = seg_start
            .equals(val(0u32))
            .select_branched(|| val(0u32), || global_excl.index(seg_start).load());
          let total = global_excl.index(seg_start + capacity_i).load() - seg_start_excl;

          let args = ENode::<DrawIndirectArgsStorage> {
            vertex_count: total,
            instance_count: val(1),
            base_vertex: val(0),
            base_instance: val(0),
          }
          .construct();
          output_indirect.index(i).store(args);
        });
      });

      builder
    });

    BindingBuilder::default()
      .with_bind(&global_excl)
      .with_bind(&input.list_info.sub_list_ranges)
      .with_bind(&input.list_info.sum_all_count)
      .with_bind(&per_sub_prefix_flat)
      .with_bind(&per_sub_indirect_flat)
      .with_bind(&padded_stride_info)
      .with_bind(&aligned_counts)
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups_indirect_by_buffer_resource_view(&dispatch_indirect_view);
  });

  // ---- Build per-sub-list helpers and draw commands ----
  let mut prefix_offsets = Vec::with_capacity(num_sub_lists as usize);
  let mut running = 0u32;
  for i in 0..num_sub_lists as usize {
    prefix_offsets.push(running);
    running += padded_stride_info_vec[1 + i];
  }

  let prefix_buffer_resource = &per_sub_prefix_flat.gpu.resource;

  let mut results = Vec::with_capacity(num_sub_lists as usize);
  for (i, info) in input.list_info.sub_list_infos.iter().enumerate() {
    // Create count view: each count is at aligned offset i * align_bytes in the aligned_counts buffer
    let count_offset = i as u64 * align_bytes;
    debug_assert!(count_offset.is_multiple_of(align_bytes));
    let count_view = aligned_counts.gpu.resource.create_view(GPUBufferViewRange {
      offset: count_offset,
      size: std::num::NonZeroU64::new(4).into(),
    });

    // Create prefix sum view: slice the flat prefix buffer at the padded offset
    let offset = prefix_offsets[i] as u64 * 4; // 4 bytes per u32;
    debug_assert!(offset.is_multiple_of(align_bytes));
    let prefix_view = prefix_buffer_resource.create_view(GPUBufferViewRange {
      offset,
      size: std::num::NonZeroU64::new((info.capacity + 1) as u64 * 4).into(),
    });
    let prefix_abstract: AbstractReadonlyStorageBuffer<[u32]> =
      StorageBufferReadonlyDataView::<[u32]>::try_from_raw(prefix_view)
        .unwrap()
        .into();

    // Create draw count view as abstract buffer
    let draw_count_abstract: AbstractReadonlyStorageBuffer<u32> =
      StorageBufferReadonlyDataView::<u32>::try_from_raw(count_view)
        .unwrap()
        .into();

    // Create command pool slice view for this sub-list
    let cmd_view =
      create_command_pool_slice_view(&input.command_pool, info.offset, info.capacity, align_bytes);

    let draw_commands = match &input.command_pool {
      StorageDrawCommands::Indexed(_) => {
        let view =
          StorageBufferReadonlyDataView::<[DrawIndexedIndirectArgsStorage]>::try_from_raw(cmd_view)
            .unwrap();
        StorageDrawCommands::Indexed(view.into())
      }
      StorageDrawCommands::NoneIndexed(_) => {
        let view =
          StorageBufferReadonlyDataView::<[DrawIndirectArgsStorage]>::try_from_raw(cmd_view)
            .unwrap();
        StorageDrawCommands::NoneIndexed(view.into())
      }
    };

    let helper = DowngradeMultiIndirectDrawCountHelper {
      sub_draw_range_start_prefix_sum: prefix_abstract,
      draw_count: draw_count_abstract,
      draw_commands,
    };

    // Create indirect arg view for this sub-list (1 element)
    let indirect_view = per_sub_indirect_flat
      .gpu
      .resource
      .create_view(GPUBufferViewRange {
        offset: i as u64 * std::mem::size_of::<DrawIndirectArgsStorage>() as u64,
        size: std::num::NonZeroU64::new(std::mem::size_of::<DrawIndirectArgsStorage>() as u64)
          .into(),
      });

    let cmd = DrawCommand::Indirect {
      indirect_buffer: indirect_view,
      indexed: false, // downgraded draw is always non-indexed indirect
    };

    results.push((helper, cmd));
  }

  results
}

/// Extract the vertex_count of a single draw command, given the global draw command index
/// within a list pool. Binary-searches sub_list_ranges to find the pool offset.
#[derive(Clone)]
struct ListPoolVertexCountSource {
  command_pool: StorageDrawCommands,
  sub_list_ranges: StorageBufferReadonlyDataView<[Vec4<u32>]>,
  sum_all_count: StorageBufferReadonlyDataView<u32>,
  total_capacity: u32,
}

impl ShaderHashProvider for ListPoolVertexCountSource {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.command_pool.is_index().hash(hasher);
  }
}

impl ComputeComponentIO<u32> for ListPoolVertexCountSource {}

impl ComputeComponent<Node<u32>> for ListPoolVertexCountSource {
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn work_size(&self) -> Option<u32> {
    None
  }

  fn result_size(&self) -> u32 {
    self.total_capacity
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let command_pool = self.command_pool.build(builder.bindgroups());
    let sub_list_ranges = builder.bind_by(&self.sub_list_ranges);
    let sum_all_count = builder.bind_by(&self.sum_all_count);

    Box::new(ListPoolVertexCountInvocation {
      command_pool,
      sub_list_ranges,
      sum_all_count,
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.command_pool.bind(builder);
    builder.bind(&self.sub_list_ranges);
    builder.bind(&self.sum_all_count);
  }
}

struct ListPoolVertexCountInvocation {
  command_pool: StorageDrawCommandsInvocation,
  sub_list_ranges: ShaderReadonlyPtrOf<[Vec4<u32>]>,
  sum_all_count: ShaderReadonlyPtrOf<u32>,
}

impl DeviceInvocation<Node<u32>> for ListPoolVertexCountInvocation {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
    let global_id = id.x();
    let size_all = self.sum_all_count.load();
    let in_bound = global_id.less_than(size_all);

    // Binary search for sub-list containing global_id (same pattern as DeviceDrawListInvocation)
    let sub_list_count = self.sub_list_ranges.array_length();
    let low = val(0u32).make_local_var();
    let high = sub_list_count.make_local_var();
    let found = val(0u32).make_local_var();

    loop_by(|cx| {
      let lo = low.load();
      let hi = high.load();
      let done = lo.greater_than(hi).or(lo.equals(hi));
      if_by(done, || cx.do_break());

      let mid = (lo + hi) / val(2u32);
      let z_mid = self.sub_list_ranges.index(mid).load().z();

      let p_le_id = z_mid.less_than(global_id).or(z_mid.equals(global_id));
      if_by(p_le_id, || {
        found.store(mid);
        low.store(mid + val(1u32));
      })
      .else_by(|| {
        high.store(mid);
      });
    });

    let list_idx = found.load();

    let result = in_bound.not().select_branched(
      || zeroed_val(),
      || {
        let range = self.sub_list_ranges.index(list_idx).load();
        let offset = range.x();
        let base = range.z();
        let pool_index = global_id - base + offset;
        self.command_pool.vertex_count(pool_index)
      },
    );

    (result, in_bound)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.sum_all_count.load(), val(0), val(0)).into()
  }
}

/// Create a GPUBufferResourceView that slices a StorageDrawCommands buffer by offset and count.
fn create_command_pool_slice_view(
  pool: &StorageDrawCommands,
  offset: u32,
  count: u32,
  align_check: u64,
) -> GPUBufferResourceView {
  let item_size = match pool {
    StorageDrawCommands::Indexed(_) => std::mem::size_of::<DrawIndexedIndirectArgsStorage>(),
    StorageDrawCommands::NoneIndexed(_) => std::mem::size_of::<DrawIndirectArgsStorage>(),
  } as u64;

  let raw_view = pool.indirect_buffer();

  let offset = offset as u64 * item_size;
  debug_assert!(offset.is_multiple_of(align_check));

  raw_view.resource.create_view(GPUBufferViewRange {
    offset,
    size: std::num::NonZeroU64::new(count as u64 * item_size).into(),
  })
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
    let ranges_init = vec![Vec4::new(0, max_count, 0, 0)];
    let sub_list_ranges = create_gpu_readonly_storage(ranges_init.as_slice(), &cx.gpu);

    let list_info = MultiRangeDispatchInfo {
      sub_list_ranges,
      sum_all_count: draw_count,
      sub_list_infos: vec![SubListHostInfo {
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

pub struct DowngradeMultiIndirectDrawCountHelper {
  pub(crate) sub_draw_range_start_prefix_sum: AbstractReadonlyStorageBuffer<[u32]>,
  pub(crate) draw_count: AbstractReadonlyStorageBuffer<u32>,
  pub(crate) draw_commands: StorageDrawCommands,
}

impl ShaderHashProvider for DowngradeMultiIndirectDrawCountHelper {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.draw_commands.is_index().hash(hasher);
  }
}

impl DowngradeMultiIndirectDrawCountHelper {
  pub fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> DowngradeMultiIndirectDrawCountHelperInvocation {
    DowngradeMultiIndirectDrawCountHelperInvocation {
      sub_draw_range_start_prefix_sum: cx.bind_by(&self.sub_draw_range_start_prefix_sum),
      draw_commands: self.draw_commands.build(cx),
      real_draw_command_count: cx.bind_by(&self.draw_count).load(),
    }
  }
  pub fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.sub_draw_range_start_prefix_sum);
    self.draw_commands.bind(builder);
    builder.bind(&self.draw_count);
  }
}

pub struct DowngradeMultiIndirectDrawCountHelperInvocation {
  sub_draw_range_start_prefix_sum: ShaderReadonlyPtrOf<[u32]>,
  real_draw_command_count: Node<u32>,
  draw_commands: StorageDrawCommandsInvocation,
}

pub struct MultiDrawDowngradeVertexInfo {
  pub sub_draw_command_idx: Node<u32>,
  pub vertex_index_inside_sub_draw: Node<u32>,
  pub base_vertex_or_index_offset_for_sub_draw: Node<u32>,
  pub base_instance: Node<u32>,
}

impl DowngradeMultiIndirectDrawCountHelperInvocation {
  pub fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
    let vertex_index = builder.query::<VertexIndex>();

    let MultiDrawDowngradeVertexInfo {
      sub_draw_command_idx: _,
      vertex_index_inside_sub_draw,
      base_vertex_or_index_offset_for_sub_draw,
      base_instance,
    } = self.get_current_vertex_draw_info(vertex_index);

    builder.register::<VertexIndexForMIDCDowngrade>(
      vertex_index_inside_sub_draw + base_vertex_or_index_offset_for_sub_draw,
    );
    builder.register::<VertexIndexForMIDCDowngradeRelative>(vertex_index_inside_sub_draw);

    builder.register::<VertexInstanceIndex>(base_instance);

    base_instance
  }

  fn get_current_vertex_draw_info(&self, vertex_id: Node<u32>) -> MultiDrawDowngradeVertexInfo {
    // binary search for current draw command
    let start = val(0_u32).make_local_var();
    let end = (self.real_draw_command_count - val(1)).make_local_var();

    loop_by(|cx| {
      if_by(start.load().greater_equal_than(end.load()), || {
        cx.do_break()
      });

      let mid = (start.load() + end.load()) / val(2);
      let test = self
        .sub_draw_range_start_prefix_sum
        .index(mid + val(1))
        .load();
      if_by(test.less_equal_than(vertex_id), || {
        start.store(mid + val(1)); // in [mid+ 1, end]
      })
      .else_by(|| {
        end.store(mid); // in [start, mid]
      });
    });

    let index = start.load();
    let draw_base_offset = self.sub_draw_range_start_prefix_sum.index(index).load();
    let draw_inner_offset = vertex_id - draw_base_offset;

    let (offset, base_instance) = match &self.draw_commands {
      StorageDrawCommandsInvocation::Indexed(cmds) => {
        let draw_cmd = cmds.index(index);
        let offset = draw_cmd.base_index().load();
        let base_instance = draw_cmd.base_instance().load();
        (offset, base_instance)
      }
      StorageDrawCommandsInvocation::NoneIndexed(cmds) => {
        let draw_cmd = cmds.index(index);
        let offset = draw_cmd.base_vertex().load();
        let base_instance = draw_cmd.base_instance().load();
        (offset, base_instance)
      }
    };

    MultiDrawDowngradeVertexInfo {
      sub_draw_command_idx: index,
      vertex_index_inside_sub_draw: draw_inner_offset,
      base_vertex_or_index_offset_for_sub_draw: offset,
      base_instance,
    }
  }
}

pub struct MidcDowngradeWrapperForIndirectMeshSystem<T> {
  pub mesh_system: T,
  pub enable_downgrade: bool,
  pub index: Option<AbstractReadonlyStorageBuffer<[u32]>>,
}

impl<T: ShaderHashProvider + 'static> ShaderHashProvider
  for MidcDowngradeWrapperForIndirectMeshSystem<T>
{
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.mesh_system.hash_pipeline(hasher);
    self.enable_downgrade.hash(hasher);
  }
}

impl<T> GraphicsShaderProvider for MidcDowngradeWrapperForIndirectMeshSystem<T>
where
  T: GraphicsShaderProvider,
{
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      // here we override the builtin
      if self.enable_downgrade {
        if let Some(index) = &self.index {
          let vertex_real_index = vertex.query::<VertexIndexForMIDCDowngrade>();
          let index_pool = binding.bind_by(index);
          let index = index_pool.index(vertex_real_index).load();
          vertex.register::<VertexIndex>(index);
        } else {
          let relative = vertex.query::<VertexIndexForMIDCDowngradeRelative>();
          vertex.register::<VertexIndex>(relative);
        }
      }
    });
    self.mesh_system.build(builder);
  }
}

impl<T: ShaderPassBuilder> ShaderPassBuilder for MidcDowngradeWrapperForIndirectMeshSystem<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some(index) = &self.index {
      // when midc downgrade enabled, the index multi draw will be downgraded into single none index draw,
      // so we use storage binding for index buffer
      if self.enable_downgrade {
        ctx.binding.bind(index);
      } else {
        let index = index.get_gpu_buffer_view().unwrap();
        ctx
          .pass
          .set_index_buffer_by_buffer_resource_view(&index, IndexFormat::Uint32);
      }
    }
    self.mesh_system.setup_pass(ctx);
  }
}

#[cfg(test)]
mod tests;
