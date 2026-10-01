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

  /// the previous building stage is restored after the call
  pub fn expect_task_shader(&mut self, f: impl FnOnce(&mut ShaderTaskBuilder)) {
    let previous = get_current_stage();
    set_current_building(ShaderStage::Task.into());
    f(self.task.as_mut().unwrap());
    set_current_building(previous);
  }

  /// the previous building stage is restored after the call
  pub fn mesh_shader(&mut self, f: impl FnOnce(&mut ShaderMeshBuilder)) {
    let previous = get_current_stage();
    set_current_building(ShaderStage::Mesh.into());
    f(&mut self.mesh);
    set_current_building(previous);
  }

  /// set the mesh stage as the current building stage, for the
  /// [AbstractShaderVertexBuilder::set_current_building] implementation of the mesh pipeline, so the
  /// vertex logic is built in the mesh stage by default
  pub fn set_mesh_as_current_building(&self) {
    set_current_building(ShaderStage::Mesh.into());
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

  /// the task stage is compute like, the workgroup size must be configured
  ///
  /// assume called in task scope
  pub fn config_work_group_size(&mut self, size: impl IntoWorkgroupSize) {
    call_shader_api(|api| api.set_workgroup_size(size.into_size()));
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
  /// the mesh stage is compute like, the workgroup size must be configured
  ///
  /// assume called in mesh scope
  pub fn config_work_group_size(&mut self, size: impl IntoWorkgroupSize) {
    call_shader_api(|api| api.set_workgroup_size(size.into_size()));
  }

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
      host_layout: None,
    });

    let vertex_output_type = ShaderSizedValueType::Struct(vertex_output_type);

    // the output variable holds the arrays, the mesh stage info describes their element types
    let primitive_output_array = ShaderSizedValueType::FixedSizeArray(
      Box::new(primitive_output_type.clone()),
      max_primitives as usize,
    );
    let vertex_output_array = ShaderSizedValueType::FixedSizeArray(
      Box::new(vertex_output_type.clone()),
      max_vertices as usize,
    );

    let mesh_shader_output_all_ty = ShaderSizedValueType::Struct(ShaderStructMetaInfo {
      name: "MeshShaderOutput".into(),
      fields: vec![
        ShaderStructFieldMetaInfo {
          name: "vertices".into(),
          ty: vertex_output_array,
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshVerticesOutput).into(),
        },
        ShaderStructFieldMetaInfo {
          name: "primitives".into(),
          ty: primitive_output_array,
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
      host_layout: None,
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

      // the vertex outputs are stored in the local variables, see set_vertex_out_impl
      for (node, _) in self.io_mapping.vertex_out.values() {
        parameters[node.location] = api.load(node.node);
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

  pub fn mark_position_invariant(&mut self) {
    self.io_mapping.position_invariant = true;
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

  let p = ShaderBuiltInDecorator::VertexPositionOut {
    invariant: io_mapping.position_invariant,
  };
  fields.push(ShaderStructFieldMetaInfo {
    name: "position".into(),
    ty: ShaderSizedValueType::Primitive(p.data_ty().unwrap()),
    ty_deco: Some(ShaderFieldDecorator::BuiltIn(p)),
  });

  ShaderStructMetaInfo {
    name: "MeshShaderVertexOutput".into(),
    fields,
    host_layout: None,
  }
}
