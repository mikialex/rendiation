use crate::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct CommonMeshBuffer {
  pub indices: Vec<u32>,
  pub vertices: Vec<CommonVertex>,
}

impl CommonMeshBuffer {
  pub fn deduplicate_indices_and_remove_unused_vertices(self) -> Self {
    let (indices, vertices) = create_deduplicated_index_vertex_mesh(
      self.indices.iter().map(|i| self.vertices[*i as usize]),
    );
    Self { indices, vertices }
  }
}
