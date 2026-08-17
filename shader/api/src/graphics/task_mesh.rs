use crate::*;

pub trait MeshShaderLogic {
  fn has_task_stage(&self) -> bool;
  fn create_abstract_vertex_shader_cx(
    &self,
    group: ShaderTaskMeshBuilderGroup,
  ) -> Box<dyn AbstractShaderVertexBuilder>;
}

pub struct ShaderTaskMeshBuilderGroup {
  task: Option<ShaderTaskBuilder>,
  mesh: ShaderMeshBuilder,
}

impl ShaderTaskMeshBuilderGroup {
  pub(crate) fn new(has_task_stage: bool) -> Self {
    Self {
      task: has_task_stage.then_some(ShaderTaskBuilder {}),
      mesh: ShaderMeshBuilder {
        registry: Default::default(),
        primitive_state: default_primitive_state(),
      },
    }
  }

  pub fn expect_task_shader(&mut self, f: impl FnOnce(&mut ShaderTaskBuilder)) {
    set_current_building(ShaderStage::Task.into());
    f(self.task.as_mut().unwrap());
    set_current_building(None);
  }

  pub fn mesh_shader(&mut self, f: impl FnOnce(&mut ShaderMeshBuilder)) {
    set_current_building(ShaderStage::Mesh.into());
    f(&mut self.mesh);
    set_current_building(None);
  }
}

pub struct ShaderTaskBuilder {}

impl ShaderTaskBuilder {
  /// assume called in task scope
  pub fn define_task_payload_output<P: ShaderSizedValueNodeType>(&mut self) -> ShaderPtrOf<P> {
    define_task_payload::<P>()
  }

  pub fn set_output_mesh_task_size(&mut self, size: Node<Vec3<u32>>) {
    call_shader_api(|api| api.set_output_mesh_task_size(size.handle()));
  }
}

pub struct ShaderMeshBuilder {
  pub registry: SemanticRegistry,
  pub primitive_state: PrimitiveState,
}

fn define_task_payload<P: ShaderSizedValueNodeType>() -> ShaderPtrOf<P> {
  let output_variable = ShaderInputNode::TaskPayload { ty: P::sized_ty() }.insert_api_raw();

  call_shader_api(|api| {
    api.define_task_payload_io(output_variable);
  });
  P::create_view_from_raw_ptr(Box::new(output_variable))
}

impl ShaderMeshBuilder {
  /// the P must match the task shader defined output
  ///
  /// assume called in mesh scope
  pub fn expect_task_input_input<P: ShaderSizedValueNodeType>(&mut self) -> ShaderPtrOf<P> {
    define_task_payload::<P>()
  }

  /// return the handle to shared-mem node for data write
  ///
  /// assume called in mesh scope
  pub fn define_mesh_output_info(
    &mut self,
    topology: MeshOutputTopology,
    max_vertices: u32,
    max_primitives: u32,
    vertex_output_type: ShaderStructMetaInfo,
  ) -> ShaderNodeRawHandle {
    // todo, support user defined per primitive output
    let primitive_output_type = ShaderSizedValueType::Struct(ShaderStructMetaInfo {
      name: "MeshShaderPrimitiveOutput".into(),
      fields: vec![ShaderStructFieldMetaInfo {
        name: topology.as_struct_field_name().into(),
        ty: ShaderSizedValueType::Primitive(topology.data_type()),
        ty_deco: Some(ShaderFieldDecorator::BuiltIn(topology.deco())),
      }],
    });

    let primitive_output_type = ShaderSizedValueType::FixedSizeArray(
      Box::new(primitive_output_type),
      max_primitives as usize,
    );

    let vertex_output_type = ShaderSizedValueType::Struct(vertex_output_type);

    let vertex_output_type =
      ShaderSizedValueType::FixedSizeArray(Box::new(vertex_output_type), max_vertices as usize);

    let mesh_shader_output_all_ty = ShaderSizedValueType::Struct(ShaderStructMetaInfo {
      name: "MeshShaderOutput".into(),
      fields: vec![
        ShaderStructFieldMetaInfo {
          name: "vertices".into(),
          ty: vertex_output_type.clone(),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshVerticesOutput).into(),
        },
        ShaderStructFieldMetaInfo {
          name: "primitives".into(),
          ty: primitive_output_type.clone(),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshPrimitiveOutput)
            .into(),
        },
        ShaderStructFieldMetaInfo {
          name: "vertex_count".into(),
          ty: ShaderSizedValueType::Primitive(PrimitiveShaderValueType::u32()),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshVertexCount).into(),
        },
        ShaderStructFieldMetaInfo {
          name: "primitive_count".into(),
          ty: ShaderSizedValueType::Primitive(PrimitiveShaderValueType::u32()),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshPrimitiveCount).into(),
        },
      ],
    });

    let output_variable = ShaderInputNode::WorkGroupShared {
      ty: mesh_shader_output_all_ty,
    }
    .insert_api_raw();

    call_shader_api(|api| {
      api.define_mesh_info(MeshStageInfo {
        topology,
        max_vertices,
        max_primitives,
        vertex_output_type,
        primitive_output_type,
        output_variable,
      })
    });

    output_variable
  }
}

/// this struct can be used to help impl AbstractShaderVertexBuilder in mesh pipeline.
pub struct MeshShaderVertexHelper {
  pub max_vertices: u32,
  pub max_primitives: u32,
  pub vertices_count: Node<u32>,
  pub primitives_count: Node<u32>,

  pub io_mapping: ShapeFragmentIOMapping,
}

impl MeshShaderVertexHelper {
  pub fn new(
    max_vertices: u32,
    max_primitives: u32,
    vertices_count: Node<u32>,
    primitives_count: Node<u32>,
  ) -> Self {
    Self {
      max_vertices,
      max_primitives,
      vertices_count,
      primitives_count,
      io_mapping: Default::default(),
    }
  }

  /// the passed in output node is shaderPtrOf<VertexOutputType>
  pub fn finalize_write(
    &mut self,
    output: ShaderNodeRawHandle,
    clip_position: ShaderNodeRawHandle,
  ) {
    call_shader_api(|api| {
      let mut parameters =
        vec![ShaderNodeRawHandle { handle: usize::MAX }; self.io_mapping.vertex_out.len()];

      for (node, _) in self.io_mapping.vertex_out.values() {
        parameters[node.location] = node.node;
      }
      parameters.push(clip_position);

      let vertex = api.make_expression(ShaderNodeExpr::Compose {
        target: ShaderSizedValueType::Struct(create_output_struct_for_mesh_vertices_output(
          &self.io_mapping,
        )),
        parameters,
      });
      api.store(vertex, output);
    });
  }

  pub fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    self.io_mapping.sync_fragment_out(fragment);
  }

  pub fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    interpolation: ShaderInterpolation,
  ) {
    self
      .io_mapping
      .set_vertex_out_impl(ty_id, ty, interpolation, &|_interpolation| {
        call_shader_api(|api| {
          let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(
            ShaderSizedValueType::Primitive(ty),
          ));
          let target = api.make_local_var(ty);
          api.store(node.handle(), target);

          target
        })
      });
  }
}

pub fn create_output_struct_for_mesh_vertices_output(
  io_mapping: &ShapeFragmentIOMapping,
) -> ShaderStructMetaInfo {
  // fields must be ordered by location to match the compose parameters order in finalize_write,
  // since vertex_out is a hash map with nondeterministic iteration order.
  let mut entries: Vec<_> = io_mapping.vertex_out.values().collect();
  entries.sort_by_key(|(info, _)| info.location);

  let mut fields: Vec<_> = entries
    .into_iter()
    .map(|(info, interpolation)| ShaderStructFieldMetaInfo {
      name: format!("field_{}", info.location),
      ty: ShaderSizedValueType::Primitive(info.ty),
      ty_deco: Some(ShaderFieldDecorator::Location(
        info.location,
        Some(*interpolation),
      )),
    })
    .collect();

  let p = ShaderBuiltInDecorator::VertexPositionOut;
  fields.push(ShaderStructFieldMetaInfo {
    name: "position".into(),
    ty: ShaderSizedValueType::Primitive(p.data_ty().unwrap()),
    ty_deco: Some(ShaderFieldDecorator::BuiltIn(p)),
  });

  ShaderStructMetaInfo {
    name: "MeshShaderVertexOutput".into(),
    fields,
  }
}
