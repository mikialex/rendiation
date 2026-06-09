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
  }
}

struct DeviceDrawListInvocation {
  scene_id_pool: ShaderReadonlyPtrOf<[u32]>,
  // (offset, count, count_prefix_sum)
  sub_list_ranges: ShaderReadonlyPtrOf<[Vec3<u32>]>,
  size_all: ShaderReadonlyPtrOf<u32>,
}

impl DeviceInvocation<Node<Vec2<u32>>> for DeviceDrawListInvocation {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<Vec2<u32>>, Node<bool>) {
    let list_index: Node<u32> = todo!(); // binary search
    let out_of_bound: Node<bool> = todo!();

    // todo, avoid out_of_bound access
    let r = out_of_bound.select_branched(
      || zeroed_val(),
      || {
        let offset = self.sub_list_ranges.index(list_index).load().x(); // todo, optimize
        let base = self.sub_list_ranges.index(list_index).load().z();
        let id = self
          .scene_id_pool
          .index(logic_global_id.x() - base + offset)
          .load();

        vec2_node((id, list_index))
      },
    );

    (r, out_of_bound)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    (self.size_all.load(), val(0), val(0)).into()
  }
}
