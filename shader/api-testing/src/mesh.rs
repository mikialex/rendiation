use std::any::TypeId;

use rendiation_shader_api::*;

use crate::graphics::*;
use crate::harness::*;

both!(MeshColor, Vec4<f32>);
both!(MeshId, u32);
only_vertex!(MeshOutputVertexIndex, u32);
only_vertex!(MeshOutputPrimitiveIndex, u32);
only_vertex!(MeshOutputTriangle, Vec3<u32>);
only_vertex!(MeshLateSource, Vec2<f32>);
only_fragment!(MeshLate, Vec2<f32>);

#[repr(C)]
#[shader_struct]
#[derive(Clone, Copy)]
pub struct TestTaskPayload {
  pub color: Vec4<f32>,
  pub id: u32,
}

const MAX_VERTICES: u32 = 3;
const MAX_PRIMITIVES: u32 = 1;
const WORKGROUP_SIZE: u32 = 3;

/// The mesh shading logic of the tests. Each mesh invocation writes one vertex and one triangle,
/// the indices, the triangle and the clip position are given by the registered semantics, and
/// the vertex outputs are set like the vertex stage. The mesh workgroup size is configured if
/// given.
struct TestMeshShading {
  task: bool,
  workgroup_size: Option<u32>,
}

impl MeshShaderLogic for TestMeshShading {
  fn has_task_stage(&self) -> bool {
    self.task
  }

  fn create_abstract_vertex_shader_cx(
    &self,
    mut group: ShaderTaskMeshBuilderGroup,
  ) -> Box<dyn AbstractShaderVertexBuilder> {
    let mut counts = None;
    group.mesh_shader(|mesh| {
      if let Some(size) = self.workgroup_size {
        mesh.config_work_group_size(size);
      }
      counts = Some((val(MAX_VERTICES), val(MAX_PRIMITIVES)));
    });
    let (vertices, primitives) = counts.unwrap();
    Box::new(TestMeshBuilder {
      group,
      registry: Default::default(),
      primitive_state: default_primitive_state(),
      helper: MeshShaderVertexHelper::new(MAX_VERTICES, MAX_PRIMITIVES, vertices, primitives),
    })
  }
}

struct TestMeshBuilder {
  group: ShaderTaskMeshBuilderGroup,
  registry: SemanticRegistry,
  primitive_state: PrimitiveState,
  helper: MeshShaderVertexHelper,
}

impl AbstractShaderVertexBuilder for TestMeshBuilder {
  fn task_mesh_shader(&mut self) -> Option<&mut ShaderTaskMeshBuilderGroup> {
    Some(&mut self.group)
  }

  fn vertex_shader(&mut self) -> Option<&mut ShaderRawVertexBuilder> {
    None
  }

  fn set_current_building(&mut self) {
    self.group.set_mesh_as_current_building();
  }

  fn finalize_write(&mut self) {
    let position = self.try_query::<ClipPosition>();
    let vertex_index = self.query::<MeshOutputVertexIndex>();
    let primitive_index = self.query::<MeshOutputPrimitiveIndex>();
    let triangle = self.query::<MeshOutputTriangle>();
    let Self { group, helper, .. } = self;
    group.mesh_shader(|mesh| {
      let position = position.unwrap_or_else(zeroed_val);
      let vertex_ty = create_output_struct_for_mesh_vertices_output(&helper.io_mapping);
      let output = mesh.define_mesh_output_info(
        MeshOutputTopology::Triangles,
        MAX_VERTICES,
        MAX_PRIMITIVES,
        vertex_ty,
      );
      u32::create_view_from_raw_ptr(output.field_index(2)).store(helper.vertices_count);
      u32::create_view_from_raw_ptr(output.field_index(3)).store(helper.primitives_count);
      let primitive = output
        .field_index(1)
        .field_array_index(primitive_index)
        .field_index(0);
      Vec3::<u32>::create_view_from_raw_ptr(primitive).store(triangle);
      let vertex = output.field_index(0).field_array_index(vertex_index);
      helper.finalize_write(vertex.get_raw_ptr(), position.handle());
    });
  }

  fn sync_fragment_out(&mut self, fragment: &mut ShaderFragmentBuilder) {
    self.helper.sync_fragment_out(fragment);
  }

  fn mark_position_invariant(&mut self) {
    self.helper.mark_position_invariant();
  }

  fn set_vertex_out_impl(
    &mut self,
    ty_id: TypeId,
    ty: PrimitiveShaderValueType,
    node: NodeUntyped,
    interpolation: ShaderInterpolation,
  ) {
    let Self { group, helper, .. } = self;
    group.mesh_shader(|_| helper.set_vertex_out_impl(ty_id, ty, node, interpolation));
  }

  fn primitive_state(&mut self) -> &mut PrimitiveState {
    &mut self.primitive_state
  }

  fn registry(&mut self) -> &mut SemanticRegistry {
    &mut self.registry
  }

  fn error(&mut self, err: ShaderBuildError) {
    panic!("mesh shader build error: {err:?}")
  }
}

/// The task (if any), mesh and fragment shader modules.
struct MeshPipelineModules {
  task: Option<naga::Module>,
  mesh: naga::Module,
  fragment: naga::Module,
}

impl MeshPipelineModules {
  fn validate(&self) {
    self.task.iter().for_each(validate);
    validate(&self.mesh);
    validate(&self.fragment);
  }
}

fn build_mesh_pipeline_by(
  shading: TestMeshShading,
  logic: impl Fn(&mut ShaderRenderPipelineBuilder),
) -> MeshPipelineModules {
  let result = build_graphics_pipeline(logic, Some(&shading), &naga_stage_api);
  let VertexOrTaskMesh::TaskMesh { task, mesh } = result.shape_shader else {
    unreachable!("expect mesh pipeline")
  };
  MeshPipelineModules {
    task: task.map(naga_module),
    mesh: naga_module(mesh),
    fragment: naga_module(result.frag_shader),
  }
}

fn build_mesh_pipeline(
  task: bool,
  logic: impl Fn(&mut ShaderRenderPipelineBuilder),
) -> MeshPipelineModules {
  let shading = TestMeshShading {
    task,
    workgroup_size: Some(WORKGROUP_SIZE),
  };
  build_mesh_pipeline_by(shading, logic)
}

/// Run the logic with the mesh stage builder.
fn in_mesh_stage<T>(
  builder: &mut ShaderVertexBuilder,
  logic: impl FnOnce(&mut ShaderMeshBuilder) -> T,
) -> T {
  let mut result = None;
  builder
    .task_mesh_shader()
    .unwrap()
    .mesh_shader(|mesh| result = Some(logic(mesh)));
  result.unwrap()
}

/// The mesh stage logic of the tests: each invocation writes the vertex of its local index and
/// the first triangle, the index is returned.
fn write_mesh_vertex_and_triangle(builder: &mut ShaderVertexBuilder) -> Node<u32> {
  let (index, position, primitive, triangle) = in_mesh_stage(builder, |_| {
    let index = ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::CompLocalInvocationIndex)
      .insert_api::<u32>();
    let position: Node<Vec4<f32>> = (index.into_f32(), val(0.), val(0.), val(1.)).into();
    (index, position, val(0), val(Vec3::new(0, 1, 2)))
  });
  builder.register::<MeshOutputVertexIndex>(index);
  builder.register::<MeshOutputPrimitiveIndex>(primitive);
  builder.register::<MeshOutputTriangle>(triangle);
  builder.register::<ClipPosition>(position);
  index
}

/// The task stage logic of the tests: write the payload, and return the mesh task size computed
/// from a storage buffer.
fn write_task_payload_and_size(builder: &mut ShaderVertexBuilder) {
  builder
    .task_mesh_shader()
    .unwrap()
    .expect_task_shader(|task| {
      task.config_work_group_size(WORKGROUP_SIZE);
      let colors = fake_storage_buffer::<[Vec4<f32>]>(0);
      let payload = task.define_task_payload_output::<TestTaskPayload>();
      let index = ShaderInputNode::BuiltIn(ShaderBuiltInDecorator::CompWorkgroupId)
        .insert_api::<Vec3<u32>>()
        .x();
      payload.color().store(colors.index(index).load());
      payload.id().store(index);
      let size = (colors.array_length(), val(1), val(1)).into();
      task.set_output_mesh_task_size(size);
    });
}

/// The struct type of the mesh vertex output, it is the element type of the vertices member of
/// the mesh output variable.
fn mesh_vertex_output_members(mesh: &naga::Module) -> &[naga::StructMember] {
  let info = mesh.entry_points[0].mesh_info.as_ref().unwrap();
  let output_ty = mesh.global_variables[info.output_variable].ty;
  let naga::TypeInner::Struct { members, .. } = &mesh.types[output_ty].inner else {
    panic!("expect mesh output struct")
  };
  let vertices = members
    .iter()
    .find(|m| m.binding == Some(naga::Binding::BuiltIn(naga::BuiltIn::Vertices)))
    .unwrap();
  let naga::TypeInner::Array { base, .. } = mesh.types[vertices.ty].inner else {
    panic!("expect vertices array")
  };
  let naga::TypeInner::Struct { members, .. } = &mesh.types[base].inner else {
    panic!("expect vertex output struct")
  };
  members
}

/// the task stage writes the payload, reads a storage buffer and returns the computed mesh task
/// size
#[test]
fn task_stage() {
  let modules = build_mesh_pipeline(true, |builder| {
    builder.vertex(|builder, _| {
      write_task_payload_and_size(builder);
      write_mesh_vertex_and_triangle(builder);
    });
  });
  modules.validate();
  let task = modules.task.as_ref().unwrap();

  let entry = &task.entry_points[0];
  assert_eq!(entry.stage, naga::ShaderStage::Task);
  assert_eq!(entry.workgroup_size, [WORKGROUP_SIZE, 1, 1]);
  assert!(entry.mesh_info.is_none());
  let result = entry.function.result.as_ref().unwrap();
  assert_eq!(
    result.binding,
    Some(naga::Binding::BuiltIn(naga::BuiltIn::MeshTaskSize))
  );
  assert_eq!(
    task.types[result.ty].inner,
    naga::TypeInner::Vector {
      size: naga::VectorSize::Tri,
      scalar: naga::Scalar::U32
    }
  );
  let payload = &task.global_variables[entry.task_payload.unwrap()];
  assert_eq!(payload.space, naga::AddressSpace::TaskPayload);
  assert!(matches!(
    task.types[payload.ty].inner,
    naga::TypeInner::Struct { .. }
  ));
  assert_eq!(builtin_arguments(task), [naga::BuiltIn::WorkGroupId]);

  let mesh_entry = &modules.mesh.entry_points[0];
  assert_eq!(mesh_entry.stage, naga::ShaderStage::Mesh);
  assert!(mesh_entry.task_payload.is_none());
}

/// the task, mesh and fragment pipeline with the task payload and the vertex outputs (including
/// the one added by the fragment stage), the vertex output locations match the fragment inputs
#[test]
fn mesh_pipeline_with_task() {
  let modules = build_mesh_pipeline(true, |builder| {
    builder.vertex(|builder, _| {
      write_task_payload_and_size(builder);
      let index = write_mesh_vertex_and_triangle(builder);
      let (color, late) = in_mesh_stage(builder, |mesh| {
        let payload = mesh.expect_task_input_input::<TestTaskPayload>();
        let offsets = fake_storage_buffer::<[Vec4<f32>]>(0);
        let color = payload.color().load() + offsets.index(payload.id().load()).load();
        (color, color.xy())
      });
      builder.set_vertex_out::<MeshColor>(color);
      builder.set_vertex_out::<MeshId>(index);
      builder.register::<MeshLateSource>(late);
    });
    builder.fragment(|builder, _| {
      let color = builder.query::<MeshColor>();
      let id = builder.query::<MeshId>();
      let late = builder.query_or_interpolate_by::<MeshLate, MeshLateSource>();
      let late: Node<Vec4<f32>> = (late, late).into();
      builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      builder.store_fragment_out_vec4f(0, color * id.into_f32() + late);
    });
  });
  modules.validate();

  let mesh = &modules.mesh;
  let entry = &mesh.entry_points[0];
  assert_eq!(entry.workgroup_size, [WORKGROUP_SIZE, 1, 1]);
  assert!(entry.function.result.is_none());
  let payload = &mesh.global_variables[entry.task_payload.unwrap()];
  assert_eq!(payload.space, naga::AddressSpace::TaskPayload);

  let members = mesh_vertex_output_members(mesh);
  assert_eq!(
    builtin_members(members),
    [naga::BuiltIn::Position { invariant: false }]
  );
  let outputs = location_members(mesh, members);
  let locations: Vec<_> = outputs
    .iter()
    .map(|io| (io.location, io.interpolation))
    .collect();
  use naga::Interpolation::*;
  assert_eq!(
    locations,
    [
      (0, Some(Perspective)),
      (1, Some(Flat)),
      (2, Some(Perspective))
    ]
  );
  assert_eq!(outputs, location_arguments(&modules.fragment));
}

/// the mesh and fragment pipeline without the task stage and user defined vertex output, the
/// mesh output info describes the output variable
#[test]
fn mesh_pipeline_without_task() {
  let modules = build_mesh_pipeline(false, |builder| {
    builder.vertex(|builder, _| {
      builder.mark_position_invariant();
      write_mesh_vertex_and_triangle(builder);
    });
    builder.fragment(|builder, _| {
      let position = builder.query::<FragmentPosition>();
      builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      builder.store_fragment_out_vec4f(0, position);
    });
  });
  assert!(modules.task.is_none());
  modules.validate();

  let mesh = &modules.mesh;
  let entry = &mesh.entry_points[0];
  assert!(entry.task_payload.is_none());
  let info = entry.mesh_info.as_ref().unwrap();
  assert_eq!(info.topology, naga::MeshOutputTopology::Triangles);
  assert_eq!(info.max_vertices, MAX_VERTICES);
  assert_eq!(info.max_primitives, MAX_PRIMITIVES);
  let output = &mesh.global_variables[info.output_variable];
  assert_eq!(output.space, naga::AddressSpace::WorkGroup);

  let members = mesh_vertex_output_members(mesh);
  assert_eq!(
    builtin_members(members),
    [naga::BuiltIn::Position { invariant: true }]
  );
  assert!(location_members(mesh, members).is_empty());
  let naga::TypeInner::Struct { members, .. } = &mesh.types[info.primitive_output_type].inner
  else {
    panic!("expect primitive output struct")
  };
  assert_eq!(builtin_members(members), [naga::BuiltIn::TriangleIndices]);
}

/// the mesh stage is compute like, building without the workgroup size config is rejected
#[test]
#[should_panic(expected = "the workgroup size of the Mesh stage is not configured")]
fn mesh_pipeline_without_workgroup_size() {
  let shading = TestMeshShading {
    task: false,
    workgroup_size: None,
  };
  build_mesh_pipeline_by(shading, |builder| {
    builder.vertex(|builder, _| {
      write_mesh_vertex_and_triangle(builder);
    });
  });
}

/// the vertex stage only ability is not available in the mesh pipeline
#[test]
#[should_panic(expected = "unable to get vertex-shader-only ability")]
fn mesh_pipeline_has_no_vertex_buffer() {
  build_mesh_pipeline(false, |builder| {
    builder.vertex(|builder, _| {
      builder
        .expect_vertex_shader()
        .push_single_vertex_layout::<MeshColor>(VertexStepMode::Vertex);
    });
  });
}
