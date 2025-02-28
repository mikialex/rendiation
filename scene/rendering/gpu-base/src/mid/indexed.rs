use crate::*;

pub trait IndexedDrawCommandBuilder: ShaderHashProvider + ShaderPassBuilder + DynClone {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> DrawCommand;
  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation>;

  fn bind(&self, builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(IndexedDrawCommandBuilder);

pub trait IndexedDrawCommandBuilderInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirect>;
}

#[derive(Clone)]
pub struct IndexedDrawCommandGenerator {
  pub scene_models: Box<dyn DeviceParallelComputeIO<u32>>,
  pub generator: Box<dyn IndexedDrawCommandBuilder>,
}

impl DeviceParallelCompute<Node<DrawIndexedIndirect>> for IndexedDrawCommandGenerator {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<DrawIndexedIndirect>>> {
    Box::new(DrawCommandGeneratorComponent {
      scene_models: self.scene_models.execute_and_expose(cx),
      generator: self.generator.clone(),
    })
  }

  fn result_size(&self) -> u32 {
    self.scene_models.result_size()
  }
}
impl DeviceParallelComputeIO<DrawIndexedIndirect> for IndexedDrawCommandGenerator {}

struct DrawCommandGeneratorComponent {
  scene_models: Box<dyn DeviceInvocationComponent<Node<u32>>>,
  generator: Box<dyn IndexedDrawCommandBuilder>,
}

impl ShaderHashProvider for DrawCommandGeneratorComponent {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.scene_models.hash_pipeline(hasher);
    self.generator.hash_pipeline(hasher);
  }
}

impl DeviceInvocationComponent<Node<DrawIndexedIndirect>> for DrawCommandGeneratorComponent {
  fn work_size(&self) -> Option<u32> {
    self.scene_models.work_size()
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<DrawIndexedIndirect>>> {
    Box::new(DrawCommandGeneratorInvocation {
      scene_models: self.scene_models.build_shader(builder),
      generator: self.generator.build_invocation(builder),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    self.scene_models.bind_input(builder);
    self.generator.bind(builder);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    self.scene_models.requested_workgroup_size()
  }
}

struct DrawCommandGeneratorInvocation {
  scene_models: Box<dyn DeviceInvocation<Node<u32>>>,
  generator: Box<dyn IndexedDrawCommandBuilderInvocation>,
}

impl DeviceInvocation<Node<DrawIndexedIndirect>> for DrawCommandGeneratorInvocation {
  fn invocation_logic(
    &self,
    logic_global_id: Node<Vec3<u32>>,
  ) -> (Node<DrawIndexedIndirect>, Node<bool>) {
    let (id, valid) = self.scene_models.invocation_logic(logic_global_id);

    let draw_command = make_local_var::<DrawIndexedIndirect>();
    if_by(valid, || {
      draw_command.store(self.generator.generate_draw_command(id));
    });

    (draw_command.load(), valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.scene_models.invocation_size()
  }
}
