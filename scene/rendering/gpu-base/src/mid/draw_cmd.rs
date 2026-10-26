use crate::*;

/// The device draw command type, implemented for [DrawIndexedIndirectArgsStorage] and
/// [DrawIndirectArgsStorage].
pub trait DrawCommandStorage: Std430 + ShaderSizedValueNodeType + Clone + 'static {
  fn into_storage_draw_commands(pool: StorageBufferReadonlyDataView<[Self]>)
  -> StorageDrawCommands;
}

impl DrawCommandStorage for DrawIndexedIndirectArgsStorage {
  fn into_storage_draw_commands(
    pool: StorageBufferReadonlyDataView<[Self]>,
  ) -> StorageDrawCommands {
    StorageDrawCommands::Indexed(pool.into())
  }
}

impl DrawCommandStorage for DrawIndirectArgsStorage {
  fn into_storage_draw_commands(
    pool: StorageBufferReadonlyDataView<[Self]>,
  ) -> StorageDrawCommands {
    StorageDrawCommands::NoneIndexed(pool.into())
  }
}

/// Generate the draw command of type C for each scene model, both on host and device.
pub trait TypedDrawCommandBuilder<C>: ShaderHashProvider + DynClone {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand>;
  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn TypedDrawCommandBuilderInvocation<C>>;

  fn bind(&self, builder: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(<C> TypedDrawCommandBuilder<C>);

pub trait TypedDrawCommandBuilderInvocation<C> {
  /// the implementation must generate an empty drawcall for mesh not allocated case
  ///
  /// we don't use another compact stream pass because it's costly
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<C>;
}

impl<C: 'static> ShaderHashProvider for Box<dyn TypedDrawCommandBuilder<C>> {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher);
  }
}

/// Generate the draw command and the sub list index for each scene model in the list.
#[derive(Clone)]
pub struct DrawCommandGeneratorComponent<C> {
  pub scene_models: Box<dyn ComputeComponent<Node<Vec2<u32>>>>,
  pub generator: Box<dyn TypedDrawCommandBuilder<C>>,
}

impl<C: 'static> ShaderHashProvider for DrawCommandGeneratorComponent<C> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.scene_models.hash_pipeline_with_type_info(hasher);
    self.generator.hash_pipeline_with_type_info(hasher);
  }
}

type DrawTuple<C> = (Node<C>, Node<u32>);

impl<C: DrawCommandStorage> ComputeComponent<DrawTuple<C>> for DrawCommandGeneratorComponent<C> {
  fn work_size(&self) -> Option<u32> {
    self.scene_models.work_size()
  }
  fn result_size(&self) -> u32 {
    self.scene_models.result_size()
  }
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<DrawTuple<C>>> {
    Box::new(self.clone())
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<DrawTuple<C>>> {
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

struct DrawCommandGeneratorInvocation<C> {
  scene_models: Box<dyn DeviceInvocation<Node<Vec2<u32>>>>,
  generator: Box<dyn TypedDrawCommandBuilderInvocation<C>>,
}

impl<C: DrawCommandStorage> DeviceInvocation<DrawTuple<C>> for DrawCommandGeneratorInvocation<C> {
  fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (DrawTuple<C>, Node<bool>) {
    let (id, valid) = self.scene_models.invocation_logic(logic_global_id);

    let draw_command = make_local_var::<C>();
    if_by(valid, || {
      draw_command.store(self.generator.generate_draw_command(id.x()));
    });

    ((draw_command.load(), id.y()), valid)
  }

  fn invocation_size(&self) -> Node<Vec3<u32>> {
    self.scene_models.invocation_size()
  }
}
