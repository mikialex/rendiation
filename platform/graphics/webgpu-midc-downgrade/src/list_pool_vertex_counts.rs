use crate::*;

/// Extract the vertex_count of a single draw command, given the global draw command index
/// within a list pool. Binary-searches sub_list_ranges to find the pool offset.
#[derive(Clone)]
pub struct ListPoolVertexCountSource {
  pub command_pool: StorageDrawCommands,
  pub sub_list_ranges: StorageBufferReadonlyDataView<[StorageSubListRangeInfo]>,
  pub sum_all_count: StorageBufferReadonlyDataView<u32>,
  pub total_capacity: u32,
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
  sub_list_ranges: ShaderReadonlyPtrOf<[StorageSubListRangeInfo]>,
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
      let z_mid = self.sub_list_ranges.index(mid).count_prefix_sum().load();

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
        let range = range.expand();
        let offset = range.offset;
        let base = range.count_prefix_sum;
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
