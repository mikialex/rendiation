use rendiation_mesh_generator::*;

use crate::*;

pub fn load_widen_line_test(s_writer: &mut SceneWriter) {
  let mut writer = global_entity_of::<WideLineModelEntity>().entity_writer();

  let mesh_buffer = build_wide_line_mesh(|builder| {
    builder.build_grid_parametric(
      &SphereMeshParameter::default().make_surface(),
      TessellationConfig { u: 6, v: 6 },
      true,
    );
  });

  let wide_line_model = writer.new_entity(|w| {
    w.write::<WideLineWidth>(&5.)
      .write::<WideLineMeshBuffer>(&mesh_buffer)
  });

  let child = s_writer.create_root_child();
  s_writer.set_local_matrix(child, Mat4::translate((5., 0., 0.)));

  s_writer.model_writer.new_entity(|w| {
    w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
      .write::<SceneModelBelongsToScene>(&s_writer.scene.some_handle())
      .write::<SceneModelRefNode>(&child.some_handle())
  });
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
