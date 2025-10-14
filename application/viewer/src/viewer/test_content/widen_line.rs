use rendiation_mesh_generator::*;
use rendiation_wide_line::*;

use crate::*;

pub fn load_widen_line_test(s_writer: &mut SceneWriter) {
  let writer = global_entity_of::<WideLineModelEntity>().entity_writer();

  let mesh_buffer = build_wide_line_mesh(|builder| {
    builder.build_grid_parametric(
      &SphereMeshParameter::default().make_surface(),
      TessellationConfig { u: 6, v: 6 },
      true,
    );
  });

  let wide_line_model = writer
    .with_component_value_writer::<WideLineWidth>(5.)
    .with_component_value_writer::<WideLineMeshBuffer>(mesh_buffer)
    .new_entity();

  let child = s_writer.create_root_child();
  s_writer.set_local_matrix(child, Mat4::translate((5., 0., 0.)));

  s_writer
    .model_writer
    .component_value_writer::<SceneModelWideLineRenderPayload>(wide_line_model.some_handle())
    .component_value_writer::<SceneModelBelongsToScene>(s_writer.scene.some_handle())
    .component_value_writer::<SceneModelRefNode>(child.some_handle())
    .new_entity();
}

pub fn build_wide_line_mesh(
  f: impl FnOnce(&mut AttributesLineMeshBuilder),
) -> ExternalRefPtr<Vec<u8>> {
  let mut builder = AttributesLineMeshBuilder::default();

  f(&mut builder);

  let mesh = builder.finish();

  let line_count = mesh.mesh.primitive_count() as f32;

  let mesh: Vec<WideLineVertex> = mesh
    .mesh
    .primitive_iter()
    .enumerate()
    .map(|(i, line)| WideLineVertex {
      start: line.start.position,
      end: line.end.position,
      color: Vec4::new(1., i as f32 / line_count, 0., 1.0),
    })
    .collect();

  let u8s = bytemuck::cast_slice(&mesh);
  ExternalRefPtr::new(u8s.to_vec())
}
