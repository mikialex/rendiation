use crate::*;

/// Reads the globally prefix-scanned positions and scatters surviving elements into
/// per-sub-list compact regions of the output pool. Survival is derived from the scan
/// (positions[i] != positions[i-1]), so no separate mask buffer is needed. Threads
/// 0..K-1 additionally extract boundary values for per-sub-list metadata.
pub struct SegmentedListScatter {
  pub positions: StorageBufferReadonlyDataView<[u32]>,
  pub sub_list_ranges: StorageBufferReadonlyDataView<[StorageSubListRangeInfo]>,
  pub draw_list: DeviceDrawList,
  pub output_pool: StorageBufferDataView<[u32]>,
  pub output_ranges: StorageBufferDataView<[StorageSubListRangeInfo]>,
  pub total_count_out: StorageBufferDataView<u32>,
}

impl ShaderHashProvider for SegmentedListScatter {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.draw_list.hash_pipeline_with_type_info(hasher);
  }

  shader_hash_type_id! {}
}

impl Clone for SegmentedListScatter {
  fn clone(&self) -> Self {
    Self {
      positions: self.positions.clone(),
      sub_list_ranges: self.sub_list_ranges.clone(),
      draw_list: self.draw_list.clone(),
      output_pool: self.output_pool.clone(),
      output_ranges: self.output_ranges.clone(),
      total_count_out: self.total_count_out.clone(),
    }
  }
}

impl ComputeComponentIO<u32> for SegmentedListScatter {}

impl ComputeComponent<Node<u32>> for SegmentedListScatter {
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn work_size(&self) -> Option<u32> {
    None // indirect dispatch via draw_list's sum_all_count
  }

  fn result_size(&self) -> u32 {
    self.draw_list.dispatch_info.total_capacity
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let positions = builder.bind_by(&self.positions);
    let sub_list_ranges = builder.bind_by(&self.sub_list_ranges);
    let output_pool = builder.bind_by(&self.output_pool);
    let output_ranges = builder.bind_by(&self.output_ranges);
    let total_count_out = builder.bind_by(&self.total_count_out);

    let draw_list_inv = self.draw_list.build_shader(builder);

    Box::new(SegmentedScatterInvocation {
      draw_list: draw_list_inv,
      positions,
      sub_list_ranges,
      output_pool,
      output_ranges,
      total_count_out,
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.positions);
    builder.bind(&self.sub_list_ranges);
    builder.bind(&self.output_pool);
    builder.bind(&self.output_ranges);
    builder.bind(&self.total_count_out);
    self.draw_list.bind_input(builder);
  }
}

struct SegmentedScatterInvocation {
  draw_list: Box<dyn DeviceInvocation<Node<Vec2<u32>>>>,
  positions: ShaderReadonlyPtrOf<[u32]>,
  sub_list_ranges: ShaderReadonlyPtrOf<[StorageSubListRangeInfo]>,
  output_pool: ShaderPtrOf<[u32]>,
  output_ranges: ShaderPtrOf<[StorageSubListRangeInfo]>,
  total_count_out: ShaderPtrOf<u32>,
}

impl DeviceInvocation<Node<u32>> for SegmentedScatterInvocation {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
    let i = id.x();
    let sub_list_count = self.sub_list_ranges.array_length();

    // metadata update (threads 0..K-1) :
    //
    // Extract boundary values from the prefix-scanned positions to compute
    // per-sub-list survival counts and exclusive prefix sums.
    // Must guard against empty sub-lists (count_prefix_sum + count == 0) to
    // avoid u32 underflow when reading positions[count_prefix_sum + count - 1]
    // — this can happen after a prior culling pass produces fully-culled
    // sub-lists at the front.
    if_by(i.less_than(sub_list_count), || {
      let range = self.sub_list_ranges.index(i).load().expand();

      // Compute p_prev first (needed as fallback for p_end on empty sub-lists).
      // p_prev = inclusive prefix sum at end of previous sub-list.
      let is_first = i.equals(val(0u32));
      let p_prev = is_first.select_branched(
        || val(0u32),
        || {
          let prev_range = self.sub_list_ranges.index(i - val(1u32)).load();
          let prev_range = prev_range.expand();
          let prev_is_empty_prefix =
            (prev_range.count_prefix_sum + prev_range.count).equals(val(0u32));
          prev_is_empty_prefix.select_branched(
            || val(0u32),
            || {
              let prev_end = prev_range.count_prefix_sum + prev_range.count - val(1u32);
              self.positions.index(prev_end).load()
            },
          )
        },
      );

      // p_end = inclusive prefix sum at end of current sub-list.
      // If this (and all preceding) sub-lists are empty
      // (count_prefix_sum + count == 0), there are no elements
      // => fall back to p_prev.
      let is_empty_prefix = (range.count_prefix_sum + range.count).equals(val(0u32));
      let p_end = is_empty_prefix.select_branched(
        || p_prev,
        || {
          let end_idx = range.count_prefix_sum + range.count - val(1u32);
          self.positions.index(end_idx).load()
        },
      );

      let new_count = p_end - p_prev;
      let new_excl = p_prev;

      self.output_ranges.index(i).store(
        ENode::<StorageSubListRangeInfo> {
          offset: new_excl,
          count: new_count,
          count_prefix_sum: new_excl,
        }
        .construct(),
      );

      // The last sub-list thread also writes the total survivor count.
      let is_last = i.equals(sub_list_count - val(1u32));
      if_by(is_last, || {
        self.total_count_out.store(p_end);
      });
    });

    //  scatter (all threads)
    let (vec2, valid) = self.draw_list.invocation_logic(id);
    let model_id = vec2.x();
    let list_idx = vec2.y();

    let p_i = self.positions.index(i).load();
    // Derive keep from the inclusive prefix scan: keep[i] ⇔ p[i] != p[i-1] (i>0) or p[0]==1 (i=0)
    let is_first = i.equals(val(0u32));
    let keep = is_first.select_branched(
      || p_i.greater_than(val(0u32)),
      || p_i.greater_than(self.positions.index(i - val(1u32)).load()),
    );

    if_by(valid.and(keep), || {
      // seg_start = total survivors in sub-lists before this one.
      // sub_list_ranges[list_idx].count_prefix_sum gives the number of
      // input elements before this sub-list. If count_prefix_sum == 0
      // there are no preceding elements → seg_start = 0.
      let range = self.sub_list_ranges.index(list_idx).load().expand();
      let no_prev_elements = list_idx
        .equals(val(0u32))
        .or(range.count_prefix_sum.equals(val(0u32)));
      let seg_start = no_prev_elements.select_branched(
        || val(0u32),
        || {
          let prev_end = range.count_prefix_sum - val(1u32);
          self.positions.index(prev_end).load()
        },
      );

      let local_pos = p_i - seg_start - val(1u32);
      self
        .output_pool
        .index(seg_start + local_pos)
        .store(model_id);
    });

    (model_id, valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.draw_list.invocation_size()
  }
}
