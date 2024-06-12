use rendiation_mesh_simplification::{
  generate_vertex_remap, remap_vertex_buffer, simplify_by_edge_collapse, EdgeCollapseConfig,
};

use crate::*;

pub struct DefaultMeshLODBuilder {
  // any config goes here
}

impl MeshLodGraphBuilder for DefaultMeshLODBuilder {
  fn simplify(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
    locked_edges: &EdgeFinder,
    target_tri_num: u32,
  ) -> MeshLODGraphSimplificationResult {
    let mut vertex_lock = vec![false; vertices.len()];
    for edge in locked_edges.0.iter() {
      vertex_lock[edge.0 as usize] = true;
      vertex_lock[edge.1 as usize] = true;
    }

    let mut simplified_indices = vec![0; indices.len()];
    let result = simplify_by_edge_collapse(
      &mut simplified_indices,
      indices,
      vertices,
      Some(&vertex_lock),
      EdgeCollapseConfig {
        target_index_count: target_tri_num as usize * 3,
        target_error: f32::INFINITY, // todo, should we limit it?
        lock_border: false, /* border should be able to be simplified unless it's locked by our config */
      },
    );
    let simplified_indices = simplified_indices
      .get(0..result.result_count)
      .unwrap()
      .to_vec();

    let mut remap = vec![0; simplified_indices.len()];
    let total_vertices = generate_vertex_remap(&mut remap, Some(&simplified_indices), vertices);

    let mut result_vertices = vec![CommonVertex::default(); total_vertices];
    remap_vertex_buffer(&mut result_vertices, vertices, &remap);

    MeshLODGraphSimplificationResult {
      mesh: MeshBufferSource {
        indices: remap,
        vertices: result_vertices,
      },
      error: result.result_error,
    }
  }

  fn segment_triangles(&self, input: &MeshBufferSource) -> SegmentResult {
    todo!()
  }

  fn segment_meshlets(&self, input: &[Meshlet], adj: &MeshletAdjacencyInfo) -> SegmentResult {
    todo!()
  }
}
