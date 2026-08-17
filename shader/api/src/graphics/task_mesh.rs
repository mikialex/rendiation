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
  pub(crate) fn new(has_task_stage: bool, errors: ErrorSink) -> Self {
    Self {
      task: has_task_stage.then(|| ShaderTaskBuilder {}),
      mesh: ShaderMeshBuilder {
        registry: Default::default(),
        primitive_state: default_primitive_state(),
        errors,
        vertex_out: Default::default(),
        vertex_out_not_synced_to_fragment: Default::default(),
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
  pub fn define_task_output<T: ShaderSizedValueNodeType>(&mut self) -> ShaderPtrOf<T> {
    call_shader_api(|api| {
      api.define_task_payload_io(todo!());
    });
    todo!()
  }
}

pub struct ShaderMeshBuilder {
  registry: SemanticRegistry,
  errors: ErrorSink,
  primitive_state: PrimitiveState,

  vertex_out: FastHashMap<TypeId, (VertexIOInfo, ShaderInterpolation)>,
  vertex_out_not_synced_to_fragment: FastHashSet<TypeId>,
}

// struct PendingMeshOutputInfo {
//   topology: MeshOutputTopology,
//   max_vertices: u32,
//   max_primitives: u32,
//   vertex_output_var: ShaderNodeRawHandle,
//   primitive_output_var: ShaderNodeRawHandle,
//   vertex_count_var: ShaderNodeRawHandle,
//   primitive_count_var: ShaderNodeRawHandle,
// }

impl ShaderMeshBuilder {
  /// the P must match the task shader defined output
  pub fn expect_task_input<P: ShaderSizedValueNodeType>(&mut self) -> ShaderPtrOf<P> {
    call_shader_api(|api| {
      api.define_task_payload_io(todo!());
    });
    todo!()
  }

  pub fn define_mesh_output_info(
    &mut self,
    topology: MeshOutputTopology,
    max_vertices: u32,
    max_primitives: u32,
  ) {
    let (primitive_output_name, primitive_output_size, primitive_output_deco) = match topology {
      MeshOutputTopology::Points => (
        "point_indices",
        None,
        ShaderBuiltInDecorator::MeshPrimitivePointIndex,
      ),
      MeshOutputTopology::Lines => (
        "line_indices",
        Some(VectorSize::Bi),
        ShaderBuiltInDecorator::MeshPrimitiveLineIndex,
      ),
      MeshOutputTopology::Triangles => (
        "triangle_indices",
        Some(VectorSize::Tri),
        ShaderBuiltInDecorator::MeshPrimitiveTriangleIndex,
      ),
    };

    // todo, support user defined per primitive output
    let hardcoded_primitive_output_ty = ShaderSizedValueType::Struct(ShaderStructMetaInfo {
      name: "MeshShaderPrimitiveOutput".into(),
      fields: vec![ShaderStructFieldMetaInfo {
        name: primitive_output_name.into(),
        ty: ShaderSizedValueType::Primitive(
          if let Some(primitive_output_size) = primitive_output_size {
            PrimitiveShaderValueType::Vector {
              size: primitive_output_size,
              scalar: ScalarType::U32,
            }
          } else {
            PrimitiveShaderValueType::Scalar(ScalarType::U32)
          },
        ),
        ty_deco: Some(ShaderFieldDecorator::BuiltIn(primitive_output_deco)),
      }],
    });

    let vertex_output_type = ShaderSizedValueType::Struct(ShaderStructMetaInfo {
      name: "MeshShaderVertexOutput".into(),
      fields: self
        .vertex_out
        .iter()
        .map(|(_, (info, interpolation))| ShaderStructFieldMetaInfo {
          name: format!("field_{}", info.location),
          ty: ShaderSizedValueType::Primitive(info.ty),
          ty_deco: Some(ShaderFieldDecorator::Location(
            info.location,
            Some(*interpolation),
          )),
        })
        .collect(),
    });

    let mesh_shader_output_all_ty = ShaderSizedValueType::Struct(ShaderStructMetaInfo {
      name: "MeshShaderOutput".into(),
      fields: vec![
        ShaderStructFieldMetaInfo {
          name: "vertices".into(),
          ty: todo!(),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshVerticesOutput).into(),
        },
        ShaderStructFieldMetaInfo {
          name: "primitives".into(),
          ty: todo!(),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshPrimitiveOutput)
            .into(),
        },
        ShaderStructFieldMetaInfo {
          name: "vertex_count".into(),
          ty: ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Scalar(ScalarType::U32)),
          ty_deco: ShaderFieldDecorator::BuiltIn(ShaderBuiltInDecorator::MeshVertexCount).into(),
        },
        ShaderStructFieldMetaInfo {
          name: "primitive_count".into(),
          ty: ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Scalar(ScalarType::U32)),
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
        primitive_output_type: hardcoded_primitive_output_ty,
        output_variable: todo!(),
      })
    });
  }
}

/// this struct can be used to help impl AbstractShaderVertexBuilder in mesh pipeline.
pub struct MeshShaderVertexHelper {
  pub max_vertices: u32,
  pub max_primitives: u32,
  pub vertices_count: Node<u32>,
  pub primitives_count: Node<u32>,

  // user defined vertex out
  vertex_out: FastHashMap<TypeId, (VertexIOInfo, ShaderInterpolation)>,
  vertex_out_not_synced_to_fragment: FastHashSet<TypeId>,
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
      vertex_out: Default::default(),
      vertex_out_not_synced_to_fragment: Default::default(),
    }
  }

  pub fn finalize_write(&mut self) {
    // do the real mesh output write.
    // mesh.define_mesh_output_info(topology, max_vertices, max_primitives);
  }

  pub fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    let vertex_out = &mut self.vertex_out;
    self
      .vertex_out_not_synced_to_fragment
      .drain()
      .for_each(|id| {
        let (VertexIOInfo { ty, location, .. }, interpolation) = *vertex_out.get(&id).unwrap();

        set_current_building(ShaderStage::Fragment.into());
        let node = ShaderInputNode::UserDefinedIn {
          ty,
          location,
          interpolation: Some(interpolation),
        }
        .insert_api();
        fragment.registry.register_raw(id, node);
        set_current_building(None);

        fragment
          .fragment_in
          .insert(id, (node, ty, interpolation, location));
      })
  }

  pub fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    mut interpolation: ShaderInterpolation,
  ) {
    let location = self.vertex_out.len();
    let target = self
      .vertex_out
      .entry(ty_id)
      .or_insert_with(|| {
        if !ty.vertex_out_could_interpolated() {
          interpolation = ShaderInterpolation::Flat
        }
        // using a local var instead of define vertex shader output natively
        let node = call_shader_api(|api| {
          let ty = ShaderValueType::Single(ShaderValueSingleType::Sized(
            ShaderSizedValueType::Primitive(ty),
          ));
          api.make_local_var(ty)
        });

        (VertexIOInfo { node, ty, location }, interpolation)
      })
      .0
      .node;
    call_shader_api(|api| api.store(node.handle(), target));

    self.vertex_out_not_synced_to_fragment.insert(ty_id);
  }
}
