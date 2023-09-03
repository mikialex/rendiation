pub struct TriangleAdjacency {
  pub counts: Vec<u32>,
  offsets: Vec<u32>,
  data: Vec<u32>,
}

impl TriangleAdjacency {
  pub fn new(indices: &[u32], vertex_count: usize) -> Self {
    let mut adjacency = Self {
      counts: vec![0; vertex_count],
      offsets: vec![0; vertex_count],
      data: vec![Default::default(); indices.len()],
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
      adjacency.data[adjacency.offsets[*a as usize] as usize] = i as u32;
      adjacency.data[adjacency.offsets[*b as usize] as usize] = i as u32;
      adjacency.data[adjacency.offsets[*c as usize] as usize] = i as u32;

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
    self.data.get(start..start + count).unwrap().iter().copied()
  }
}
