use rendiation_mesh_generator::*;

use crate::*;

pub fn test_mesh_lod_graph(_writer: &mut SceneWriter) {
  {
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
      .new_entity();
  }
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
  };
  let mesh = DefaultMeshLODBuilder {}.build_from_mesh(mesh);
  ExternalRefPtr::new(mesh)
}
