use crate::*;

#[derive(Clone)]
pub struct SceneModelCulling {
  pub culler: Box<dyn AbstractCullerProvider>,
  pub input: Box<dyn DeviceParallelComputeIO<u32>>,
}

impl DeviceParallelCompute<Node<bool>> for SceneModelCulling {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<bool>>> {
    Box::new(SceneModelCullingComponent {
      culler: self.culler.clone(),
      input: self.input.execute_and_expose(cx),
    })
  }

  fn result_size(&self) -> u32 {
    self.input.result_size()
  }
}
impl DeviceParallelComputeIO<bool> for SceneModelCulling {}

struct SceneModelCullingComponent {
  culler: Box<dyn AbstractCullerProvider>,
  input: Box<dyn DeviceInvocationComponent<Node<u32>>>,
}

impl ShaderHashProvider for SceneModelCullingComponent {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.culler.hash_pipeline_with_type_info(hasher);
    self.input.hash_pipeline_with_type_info(hasher);
  }
}

impl DeviceInvocationComponent<Node<bool>> for SceneModelCullingComponent {
  fn work_size(&self) -> Option<u32> {
    self.input.work_size()
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
