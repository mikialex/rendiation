use crate::*;

/// Extract the vertex_count of a single draw command, given the global draw command index
/// within a list pool. Binary-searches sub_list_ranges to find the pool offset.
#[derive(Clone)]
pub struct ListPoolVertexCountSource {
  pub command_pool: StorageDrawCommands,
  pub ranges: DeviceMultiRangeDispatchInfo,
  pub total_capacity: u32,
}

impl ShaderHashProvider for ListPoolVertexCountSource {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.command_pool.is_index());
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
    let ranges = self.ranges.build_shader(builder);

    Box::new(ListPoolVertexCountInvocation {
      command_pool,
      ranges,
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.command_pool.bind(builder);
    self.ranges.bind_shader(builder);
  }
}

struct ListPoolVertexCountInvocation {
  command_pool: StorageDrawCommandsInvocation,
  ranges: DeviceMultiRangeDispatchInfoInvocation,
}

impl DeviceInvocation<Node<u32>> for ListPoolVertexCountInvocation {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
    let global_id = id.x();
    let (list_idx, in_bound) = self.ranges.compute_list_index(global_id);

    let result = in_bound.not().select_branched(
      || zeroed_val(),
      || {
        let range = self.ranges.read_range_info(list_idx);
        let offset = range.offset;
        let base = range.count_prefix_sum;
        let pool_index = global_id - base + offset;
        self.command_pool.vertex_count(pool_index)
      },
    );

    (result, in_bound)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.ranges.sum_all_count.load(), val(0), val(0)).into()
  }
}
