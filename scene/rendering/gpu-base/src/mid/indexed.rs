use crate::*;

pub trait IndexedDrawCommandBuilder: ShaderHashProvider + DynClone {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand>;
  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation>;

  fn bind(&self, builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(IndexedDrawCommandBuilder);

pub trait IndexedDrawCommandBuilderInvocation {
  /// the implementation must generate an empty drawcall for mesh not allocated case
  ///
  /// we don't use another compact stream pass because it's costly
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndexedIndirectArgsStorage>;
}

impl ShaderHashProvider for Box<dyn IndexedDrawCommandBuilder> {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher);
  }
}
impl IndexedDrawCommandBuilder for Box<dyn IndexedDrawCommandBuilder> {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    (**self).draw_command_host_access(id)
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation> {
    (**self).build_invocation(cx)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    (**self).bind(builder)
  }
}

#[derive(Clone)]
pub struct IndexedDrawCommandGeneratorComponent {
  pub scene_models: Box<dyn ComputeComponent<Node<Vec2<u32>>>>,
  pub generator: Box<dyn IndexedDrawCommandBuilder>,
}

impl ShaderHashProvider for IndexedDrawCommandGeneratorComponent {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.scene_models.hash_pipeline_with_type_info(hasher);
    self.generator.hash_pipeline_with_type_info(hasher);
  }
}

type IndexedDrawTuple = (Node<DrawIndexedIndirectArgsStorage>, Node<u32>);

impl ComputeComponent<IndexedDrawTuple> for IndexedDrawCommandGeneratorComponent {
  fn work_size(&self) -> Option<u32> {
    self.scene_models.work_size()
  }
  fn result_size(&self) -> u32 {
    self.scene_models.result_size()
  }
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<IndexedDrawTuple>> {
    Box::new(self.clone())
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<IndexedDrawTuple>> {
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
  scene_models: Box<dyn DeviceInvocation<Node<Vec2<u32>>>>,
  generator: Box<dyn IndexedDrawCommandBuilderInvocation>,
}

impl DeviceInvocation<IndexedDrawTuple> for DrawCommandGeneratorInvocation {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (IndexedDrawTuple, Node<bool>) {
    let (id, valid) = self.scene_models.invocation_logic(logic_global_id);

    let draw_command = make_local_var::<DrawIndexedIndirectArgsStorage>();
    if_by(valid, || {
      draw_command.store(self.generator.generate_draw_command(id.x()));
    });

    ((draw_command.load(), id.y()), valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.scene_models.invocation_size()
  }
}
