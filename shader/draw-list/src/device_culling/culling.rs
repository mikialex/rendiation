use crate::*;

#[derive(Clone)]
pub struct SceneModelCullingComponent {
  pub culler: Box<dyn AbstractCullerProvider>,
  pub input: Box<dyn ComputeComponent<Node<u32>>>,
}

impl ShaderHashProvider for SceneModelCullingComponent {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.culler.hash_pipeline_with_type_info(hasher);
    self.input.hash_pipeline_with_type_info(hasher);
  }
}

impl ComputeComponentIO<bool> for SceneModelCullingComponent {}
impl ComputeComponent<Node<bool>> for SceneModelCullingComponent {
  fn work_size(&self) -> Option<u32> {
    self.input.work_size()
  }

  fn result_size(&self) -> u32 {
    self.input.result_size()
  }

  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<bool>>> {
    Box::new(self.clone())
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<bool>>> {
    Box::new(SceneModelCullingInvocation {
      culler: self.culler.create_invocation(builder.bindgroups()),
      input: self.input.build_shader(builder),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.culler.bind(builder);
    self.input.bind_input(builder);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    self.input.requested_workgroup_size()
  }
}

struct SceneModelCullingInvocation {
  culler: Box<dyn AbstractCullerInvocation>,
  input: Box<dyn DeviceInvocation<Node<u32>>>,
}

impl DeviceInvocation<Node<bool>> for SceneModelCullingInvocation {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<bool>, Node<bool>) {
    let (id, valid) = self.input.invocation_logic(logic_global_id);
    let r = val(true).make_local_var();
    if_by(valid, || r.store(self.culler.cull(id).not()));
    (r.load(), valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.input.invocation_size()
  }
}
