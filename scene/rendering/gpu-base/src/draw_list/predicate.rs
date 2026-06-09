use crate::*;

/// Evaluates the culler for each entry in the draw list and produces a mask buffer
/// where each element is 1 (keep) or 0 (culled). The mask is later prefix-scanned to
/// compute per-sub-list survival counts (via boundary value subtraction, no atomics).
pub struct ListOfListsCullingPredicate {
  pub draw_list: DeviceDrawList,
  pub culler: Box<dyn AbstractCullerProvider>,
}

impl ShaderHashProvider for ListOfListsCullingPredicate {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.draw_list.hash_pipeline_with_type_info(hasher);
    self.culler.hash_pipeline_with_type_info(hasher);
  }

  shader_hash_type_id! {}
}

impl Clone for ListOfListsCullingPredicate {
  fn clone(&self) -> Self {
    Self {
      draw_list: self.draw_list.clone(),
      culler: self.culler.clone(),
    }
  }
}

impl ComputeComponentIO<u32> for ListOfListsCullingPredicate {}

impl ComputeComponent<Node<u32>> for ListOfListsCullingPredicate {
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn work_size(&self) -> Option<u32> {
    None // use indirect dispatch via DeviceDrawList's sum_all_count
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
    let draw_list_inv = self.draw_list.build_shader(builder);
    let culler_inv = self.culler.create_invocation(builder.bindgroups());

    Box::new(CullingPredicateInvocation {
      draw_list: draw_list_inv,
      culler: culler_inv,
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.draw_list.bind_input(builder);
    self.culler.bind(builder);
  }
}

struct CullingPredicateInvocation {
  draw_list: Box<dyn DeviceInvocation<Node<Vec2<u32>>>>,
  culler: Box<dyn AbstractCullerInvocation>,
}

impl DeviceInvocation<Node<u32>> for CullingPredicateInvocation {
  fn invocation_logic(&self, id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
    let (vec2, valid) = self.draw_list.invocation_logic(id);
    let model_id = vec2.x();

    let result = val(0u32).make_local_var();
    if_by(valid, || {
      let keep = self.culler.cull(model_id).not();
      result.store(keep.select(val(1u32), val(0u32)));
    });

    (result.load(), valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.draw_list.invocation_size()
  }
}
