use rendiation_mesh_generator::*;

use crate::*;

pub fn test_mesh_lod_graph(writer: &mut SceneWriter) {
  let mesh = {
    let lod_mesh_writer = global_entity_of::<LODGraphMeshEntity>().entity_writer();
    let mesh = build_lod_graph_mesh(|builder| {
      builder.triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 64, v: 64 },
        true,
      );
    });

    lod_mesh_writer
      .with_component_value_writer::<LODGraphData>(Some(mesh))
      .new_entity()
  };

  let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::splat(0.8),
    ..Default::default()
  }
  .write(&mut writer.pbr_sg_mat_writer);

  let child = writer.create_root_child();
  writer.set_local_matrix(child, Mat4::translate((-2., 0., -3.)));

  let std_model = {
    let writer: &mut EntityWriter<StandardModelEntity> = &mut writer.std_model_writer;
    writer.component_value_writer::<StandardModelRefPbrSGMaterial>(material.some_handle());
    writer.component_value_writer::<StandardModelRefLodGraphMeshEntity>(mesh.some_handle());
    writer.new_entity()
  };
  SceneModelDataView {
    model: std_model,
    scene: writer.scene,
    node: child,
  }
  .write(&mut writer.model_writer);
}

/// helper fn to quick build attribute mesh
pub fn build_lod_graph_mesh(
  f: impl FnOnce(&mut AttributesMeshBuilder),
) -> ExternalRefPtr<MeshLODGraph> {
  let mut builder = AttributesMeshBuilder::default();

  f(&mut builder);

  let mut mesh = builder.finish();
  let mesh = MeshBufferSource {
    indices: mesh.mesh.index.check_upgrade_to_u32().clone(),
    vertices: mesh.mesh.vertex,
  }
  .remap_vertex();

  let mesh = DefaultMeshLODBuilder {}.build_from_mesh(mesh);
  ExternalRefPtr::new(mesh)
}
