use crate::*;

pub struct ShaderTaskMeshBuilderGroup {
  pub task: Option<ShaderTaskBuilder>,
  pub mesh: ShaderMeshBuilder,
}

impl ShaderTaskMeshBuilderGroup {
  pub(crate) fn new(has_task_stage: bool, errors: ErrorSink) -> Self {
    Self {
      task: has_task_stage.then(|| ShaderTaskBuilder {}),
      mesh: ShaderMeshBuilder {
        registry: Default::default(),
        primitive_state: default_primitive_state(),
        errors,
      },
    }
  }
}

impl AbstractShaderVertexBuilder for ShaderTaskMeshBuilderGroup {
  fn task_mesh_shader(&mut self) -> Option<&mut ShaderTaskMeshBuilderGroup> {
    Some(self)
  }
  fn vertex_shader(&mut self) -> Option<&mut ShaderRawVertexBuilder> {
    None
  }

  fn set_current_building(&mut self) {
    set_current_building(ShaderStage::Mesh.into());
  }

  fn finalize_write(&mut self) {
    todo!()
  }

  fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    todo!()
  }

  fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    interpolation: ShaderInterpolation,
  ) {
    todo!()
  }

  fn primitive_state(&mut self) -> &mut PrimitiveState {
    &mut self.mesh.primitive_state
  }

  fn registry(&mut self) -> &mut SemanticRegistry {
    &mut self.mesh.registry
  }

  fn error(&mut self, err: ShaderBuildError) {
    self.mesh.errors.push(err);
  }
}

pub struct ShaderTaskBuilder {}

pub struct ShaderMeshBuilder {
  registry: SemanticRegistry,
  errors: ErrorSink,
  primitive_state: PrimitiveState,
}
