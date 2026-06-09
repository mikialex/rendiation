use crate::*;

/// Reads the globally prefix-scanned mask positions and scatters surviving elements
/// into per-sub-list compact regions of the output pool. Threads 0..K-1 additionally
/// extract boundary values to compute new per-sub-list counts and prefix sums without
/// any atomic operations.
pub struct SegmentedListScatter {
  pub positions: StorageBufferReadonlyDataView<[u32]>,
  pub mask: StorageBufferReadonlyDataView<[u32]>,
  pub sub_list_ranges: StorageBufferReadonlyDataView<[Vec4<u32>]>,
  pub draw_list: DeviceDrawList,
  pub output_pool: StorageBufferDataView<[u32]>,
  pub output_ranges: StorageBufferDataView<[Vec4<u32>]>,
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
      mask: self.mask.clone(),
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
    self.draw_list.dispatch_info.sum_all_count_host
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    let positions = builder.bind_by(&self.positions);
    let mask = builder.bind_by(&self.mask);
    let sub_list_ranges = builder.bind_by(&self.sub_list_ranges);
    let output_pool = builder.bind_by(&self.output_pool);
    let output_ranges = builder.bind_by(&self.output_ranges);
    let total_count_out = builder.bind_by(&self.total_count_out);

    let draw_list_inv = self.draw_list.build_shader(builder);

    Box::new(SegmentedScatterInvocation {
      draw_list: draw_list_inv,
      positions,
      mask,
      sub_list_ranges,
      output_pool,
      output_ranges,
      total_count_out,
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.positions);
    builder.bind(&self.mask);
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
  mask: ShaderReadonlyPtrOf<[u32]>,
  sub_list_ranges: ShaderReadonlyPtrOf<[Vec4<u32>]>,
  output_pool: ShaderPtrOf<[u32]>,
  output_ranges: ShaderPtrOf<[Vec4<u32>]>,
  total_count_out: ShaderPtrOf<u32>,
}

impl DeviceInvocation<Node<u32>> for SegmentedScatterInvocation {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
    let i = id.x();
    let sub_list_count = self.sub_list_ranges.array_length();

    // ---- metadata update (threads 0..K-1) ----
    // Extract boundary values from the prefix-scanned positions to compute
    // per-sub-list survival counts and exclusive prefix sums.
    if_by(i.less_than(sub_list_count), || {
      let range = self.sub_list_ranges.index(i).load();
      let end_idx = range.z() + range.y() - val(1u32);
      let p_end = self.positions.index(end_idx).load();

      // p_prev = inclusive prefix sum at end of previous sub-list
      let is_first = i.equals(val(0u32));
      let p_prev = is_first.select_branched(
        || val(0u32),
        || {
          let prev_range = self.sub_list_ranges.index(i - val(1u32)).load();
          let prev_end = prev_range.z() + prev_range.y() - val(1u32);
          self.positions.index(prev_end).load()
        },
      );

      let new_count = p_end - p_prev;
      let new_excl = p_prev;
      let orig_offset = range.x();

      self
        .output_ranges
        .index(i)
        .store(vec4_node((orig_offset, new_count, new_excl, val(0u32))));

      // The last sub-list thread also writes the total survivor count.
      let is_last = i.equals(sub_list_count - val(1u32));
      if_by(is_last, || {
        self.total_count_out.store(p_end);
      });
    });

    // ---- scatter (all threads) ----
    let (vec2, valid) = self.draw_list.invocation_logic(id);
    let model_id = vec2.x();
    let list_idx = vec2.y();

    let keep = self.mask.index(i).load().greater_than(val(0u32));

    if_by(valid.and(keep), || {
      let p_i = self.positions.index(i).load();

      // seg_start = total survivors in sub-lists before this one
      // = positions at the end index of the previous sub-list
      // sub_list_ranges[list_idx].z is the input prefix_sum (start global index),
      // so ranges[list_idx].z - 1 gives the last element of the previous sub-list.
      let is_first_list = list_idx.equals(val(0u32));
      let seg_start = is_first_list.select_branched(
        || val(0u32),
        || {
          let prev_end = self.sub_list_ranges.index(list_idx).load().z() - val(1u32);
          self.positions.index(prev_end).load()
        },
      );

      let local_pos = p_i - seg_start - val(1u32);
      let offset = self.sub_list_ranges.index(list_idx).load().x();
      self.output_pool.index(offset + local_pos).store(model_id);
    });

    (model_id, valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.draw_list.invocation_size()
  }
}
