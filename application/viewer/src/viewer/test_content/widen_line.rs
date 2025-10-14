use rendiation_mesh_generator::*;
use rendiation_wide_line::*;

use crate::*;

pub fn load_widen_line_test(s_writer: &mut SceneWriter) {
  let writer = global_entity_of::<WideLineModelEntity>().entity_writer();

  let mesh_buffer = build_wide_line_mesh(|builder| {
    builder.build_grid_parametric(
      &SphereMeshParameter::default().make_surface(),
      TessellationConfig { u: 4, v: 4 },
      true,
    );
  });

  let wide_line_model = writer
    .with_component_value_writer::<WideLineWidth>(5.)
    .with_component_value_writer::<WideLineMeshBuffer>(mesh_buffer)
    .new_entity();

  let child = s_writer.create_root_child();
  s_writer.set_local_matrix(child, Mat4::translate((4., 0., 10.)));

  s_writer
    .model_writer
    .component_value_writer::<SceneModelWideLineRenderPayload>(wide_line_model.some_handle())
    .component_value_writer::<SceneModelBelongsToScene>(s_writer.scene.some_handle())
    .component_value_writer::<SceneModelRefNode>(child.some_handle())
    .new_entity();
}

pub fn build_wide_line_mesh(
  f: impl FnOnce(&mut AttributesLineMeshBuilder),
) -> ExternalRefPtr<Vec<Vec3<f32>>> {
  let mut builder = AttributesLineMeshBuilder::default();

  f(&mut builder);

  let mesh = builder.finish();

  let iter = mesh
    .mesh
    .primitive_iter()
    .map(|line| line.map(|v| v.position));

  let mesh = NoneIndexedMesh::<LineList, Vec<Vec3<f32>>>::from_iter(iter);

  ExternalRefPtr::new(mesh.data)
}
