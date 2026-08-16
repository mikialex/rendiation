use crate::*;

pub trait MeshShaderLogic {
  fn has_task_stage(&self) -> bool;
  fn create_abstract_vertex_shader_cx(
    &self,
    group: ShaderTaskMeshBuilderGroup,
  ) -> Box<dyn AbstractShaderVertexBuilder>;
}

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
        mesh_info: None,
      },
    }
  }
}

// impl AbstractShaderVertexBuilder for ShaderTaskMeshBuilderGroup {
//   fn task_mesh_shader(&mut self) -> Option<&mut ShaderTaskMeshBuilderGroup> {
//     Some(self)
//   }
//   fn vertex_shader(&mut self) -> Option<&mut ShaderRawVertexBuilder> {
//     None
//   }

//   fn set_current_building(&mut self) {
//     set_current_building(ShaderStage::Mesh.into());
//   }

//   fn finalize_write(&mut self) {
//     todo!()
//   }

//   fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
//     todo!()
//   }

//   fn set_vertex_out_impl(
//     &mut self,
//     ty_id: TypeId,
//     ty: PrimitiveShaderValueType,
//     node: NodeUntyped,
//     interpolation: ShaderInterpolation,
//   ) {
//     todo!()
//   }

//   fn primitive_state(&mut self) -> &mut PrimitiveState {
//     &mut self.mesh.primitive_state
//   }

//   fn registry(&mut self) -> &mut SemanticRegistry {
//     &mut self.mesh.registry
//   }

//   fn error(&mut self, err: ShaderBuildError) {
//     self.mesh.errors.push(err);
//   }
// }

pub struct ShaderTaskBuilder {}

impl ShaderTaskBuilder {
  pub fn define_task_output<T: ShaderSizedValueNodeType>(&mut self) -> ShaderPtrOf<T> {
    todo!()
  }
}

pub struct ShaderMeshBuilder {
  registry: SemanticRegistry,
  errors: ErrorSink,
  primitive_state: PrimitiveState,
  mesh_info: Option<PendingMeshOutputInfo>,
}

struct PendingMeshOutputInfo {
  topology: MeshOutputTopology,
  max_vertices: u32,
  max_primitives: u32,
  vertex_output_var: ShaderNodeRawHandle,
  primitive_output_var: ShaderNodeRawHandle,
  vertex_count_var: ShaderNodeRawHandle,
  primitive_count_var: ShaderNodeRawHandle,
}

impl ShaderMeshBuilder {
  pub fn define_mesh_output_info(
    &mut self,
    topology: MeshOutputTopology,
    max_vertices: u32,
    max_primitives: u32,
  ) {
    todo!()
  }
}
