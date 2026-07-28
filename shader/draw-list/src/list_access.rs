use crate::*;

impl ShaderHashProvider for DeviceDrawList {
  shader_hash_type_id! {}
}

impl ComputeComponentIO<Vec2<u32>> for DeviceDrawList {}

impl ComputeComponent<Node<Vec2<u32>>> for DeviceDrawList {
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<Vec2<u32>>>> {
    Box::new(self.clone())
  }

  fn work_size(&self) -> Option<u32> {
    None
  }

  fn result_size(&self) -> u32 {
    self.dispatch_info.total_capacity
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<Vec2<u32>>>> {
    Box::new(DeviceDrawListInvocation {
      id_pool: builder.bind_by(&self.id_pool),
      ranges: self.dispatch_info.device_ranges.build_shader(builder),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.id_pool);
    self.dispatch_info.device_ranges.bind_shader(builder);
  }
}

struct DeviceDrawListInvocation {
  id_pool: ShaderReadonlyPtrOf<[u32]>,
  ranges: DeviceMultiRangeDispatchInfoInvocation,
}

impl DeviceInvocation<Node<Vec2<u32>>> for DeviceDrawListInvocation {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<Vec2<u32>>, Node<bool>) {
    let global_id = logic_global_id.x();
    let (list_index, in_bound) = self.ranges.compute_list_index(global_id);

    let r = in_bound.not().select_branched(
      || zeroed_val(),
      || {
        let range = self.ranges.read_range_info(list_index);
        let offset = range.offset;
        let base = range.count_prefix_sum;
        let id = self.id_pool.index(global_id - base + offset).load();

        vec2_node((id, list_index))
      },
    );

    (r, in_bound)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.ranges.sum_all_count.load(), val(0), val(0)).into()
  }
}
