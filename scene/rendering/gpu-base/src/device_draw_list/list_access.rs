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
    self.dispatch_info.sum_all_count_host
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<Vec2<u32>>>> {
    Box::new(DeviceDrawListInvocation {
      scene_id_pool: builder.bind_by(&self.scene_model_id_pool),
      sub_list_ranges: builder.bind_by(&self.dispatch_info.sub_list_ranges),
      size_all: builder.bind_by(&self.dispatch_info.sum_all_count),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.scene_model_id_pool);
    builder.bind(&self.dispatch_info.sub_list_ranges);
    builder.bind(&self.dispatch_info.sum_all_count);
  }
}

struct DeviceDrawListInvocation {
  scene_id_pool: ShaderReadonlyPtrOf<[u32]>,
  // (offset, count, count_prefix_sum, _padding) — Vec4 for 16B storage alignment
  sub_list_ranges: ShaderReadonlyPtrOf<[Vec4<u32>]>,
  size_all: ShaderReadonlyPtrOf<u32>,
}

impl DeviceInvocation<Node<Vec2<u32>>> for DeviceDrawListInvocation {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<Vec2<u32>>, Node<bool>) {
    let size_all = self.size_all.load();
    let in_bound = logic_global_id.x().less_than(size_all);

    let sub_list_count = self.sub_list_ranges.array_length();

    // Binary search for the sub-list containing logic_global_id.x()
    // Find the last index i where sub_list_ranges[i].z (prefix_sum) <= global_id
    let low = val(0u32).make_local_var();
    let high = sub_list_count.make_local_var();
    let found = val(0u32).make_local_var();

    loop_by(|cx| {
      let lo = low.load();
      let hi = high.load();
      let done = lo.greater_than(hi).or(lo.equals(hi));
      if_by(done, || cx.do_break());

      let mid = (lo + hi) / val(2u32);
      let prefix_sum = self.sub_list_ranges.index(mid).load().z();

      let p_le_id = prefix_sum
        .less_than(logic_global_id.x())
        .or(prefix_sum.equals(logic_global_id.x()));
      if_by(p_le_id, || {
        found.store(mid);
        low.store(mid + val(1u32));
      })
      .else_by(|| {
        high.store(mid);
      });
    });

    let list_index = found.load();

    let r = in_bound.not().select_branched(
      || zeroed_val(),
      || {
        let range = self.sub_list_ranges.index(list_index).load();
        let offset = range.x();
        let base = range.z();
        let id = self
          .scene_id_pool
          .index(logic_global_id.x() - base + offset)
          .load();

        vec2_node((id, list_index))
      },
    );

    (r, in_bound)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.size_all.load(), val(0), val(0)).into()
  }
}
