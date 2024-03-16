pub struct TriangleAdjacency {
  pub counts: Vec<u32>,
  pub offsets: Vec<u32>,
  pub face_ids: Vec<u32>,
}

impl TriangleAdjacency {
  pub fn new(indices: &[u32], vertex_count: usize) -> Self {
    let mut adjacency = Self {
      counts: vec![0; vertex_count],
      offsets: vec![0; vertex_count],
      face_ids: vec![Default::default(); indices.len()],
    };

    for index in indices {
      adjacency.counts[*index as usize] += 1;
    }

    // fill offset table
    let mut offset = 0;
    for (o, count) in adjacency.offsets.iter_mut().zip(adjacency.counts.iter()) {
      *o = offset;
      offset += *count;
    }

    assert_eq!(offset as usize, indices.len());

    // fill triangle data
    for (i, [a, b, c]) in indices.array_chunks::<3>().enumerate() {
      adjacency.face_ids[adjacency.offsets[*a as usize] as usize] = i as u32;
      adjacency.face_ids[adjacency.offsets[*b as usize] as usize] = i as u32;
      adjacency.face_ids[adjacency.offsets[*c as usize] as usize] = i as u32;

      adjacency.offsets[*a as usize] += 1;
      adjacency.offsets[*b as usize] += 1;
      adjacency.offsets[*c as usize] += 1;
    }

    // fix offsets that have been disturbed by the previous pass
    for (offset, count) in adjacency.offsets.iter_mut().zip(adjacency.counts.iter()) {
      assert!(*offset >= *count);
      *offset -= *count;
    }

    adjacency
  }

  /// note: the return is triangle idx
  pub fn iter_adjacency_faces(&self, vertex: usize) -> impl Iterator<Item = u32> + '_ {
    let start = self.offsets[vertex] as usize;
    let count = self.counts[vertex] as usize;
    self
      .face_ids
      .get(start..start + count)
      .unwrap()
      .iter()
      .copied()
  }

  pub fn update_by_remove_a_triangle(&mut self, triangle_idx: usize, indices: &[u32]) {
    for k in 0..3 {
      let index = indices[triangle_idx * 3 + k] as usize;

      let start = self.offsets[index] as usize;
      let count = self.counts[index] as usize;
      let neighbors = self.face_ids.get_mut(start..start + count).unwrap();
      let last = neighbors[count - 1];

      for tri in neighbors {
        if *tri as usize == triangle_idx {
          *tri = last; // we do only remove one tri so it's ok not to update last cursor
          self.counts[index] -= 1;
          break;
        }
      }
    }
  }
}
