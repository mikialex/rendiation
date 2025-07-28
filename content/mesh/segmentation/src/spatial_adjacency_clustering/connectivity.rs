use rendiation_mesh_core::*;

pub struct TriangleAdjacency {
  internal: Adjacency<u32>, // vertex_idx -> triangle idx
}

impl TriangleAdjacency {
  pub fn new(indices: &[u32], vertex_count: usize) -> Self {
    let vertices_iter = indices.iter().copied();
    let face_vertices_iter = indices
      .array_chunks::<3>()
      .enumerate()
      .flat_map(|(i, [a, b, c])| {
        // we must reject the degenerate triangle here, because when we remove triangle from self, we early exist
        // for first triangle.
        assert!(triangle_is_not_degenerated(&[a, b, c]));
        let i = i as u32;
        [(i, *a), (i, *b), (i, *c)]
      });

    Self {
      internal: Adjacency::from_iter(vertex_count, vertices_iter, face_vertices_iter),
    }
  }

  pub fn vertex_referenced_face_counts(&self) -> &[u32] {
    &self.internal.counts
  }

  /// note: the return is triangle idx
  pub fn iter_adjacency_faces(&self, index: u32) -> impl Iterator<Item = u32> + '_ {
    self.internal.iter_many_by_one(index).copied()
  }

  pub fn update_by_remove_a_triangle(&mut self, triangle_idx: usize, indices: &[u32]) {
    for k in 0..3 {
      let index = indices[triangle_idx * 3 + k];
      let removed = self
        .internal
        .try_remove_relation(&(triangle_idx as u32), index);

      assert!(removed);
    }
  }
}
