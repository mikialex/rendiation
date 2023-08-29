pub struct EdgeAdjacency {
  pub counts: Vec<u32>,
  pub offsets: Vec<u32>,
  pub data: Vec<HalfEdge>,
}

#[derive(Default, Clone, Copy)]
pub struct HalfEdge {
  pub next: u32,
  pub prev: u32,
}

impl EdgeAdjacency {
  pub fn new(indices: &[u32], vertex_count: usize) -> Self {
    let mut result = EdgeAdjacency {
      counts: vec![0; vertex_count],
      offsets: vec![0; vertex_count],
      data: vec![Default::default(); indices.len()],
    };
    result.update(indices, None);
    result
  }
  pub fn update(&mut self, indices: &[u32], remap: Option<&[u32]>) {
    self.counts.fill(0);
    let face_count = indices.len() / 3;

    // fill edge counts
    for index in indices {
      let v = if let Some(remap) = remap {
        remap[*index as usize]
      } else {
        *index
      };
      self.counts[v as usize] += 1;
    }

    // fill offset table
    let mut offset = 0;

    for (o, count) in self.offsets.iter_mut().zip(self.counts.iter()) {
      *o = offset;
      offset += *count;
    }

    assert_eq!(offset as usize, indices.len());

    // fill edge data
    for i in 0..face_count {
      let mut a = indices[i * 3] as usize;
      let mut b = indices[i * 3 + 1] as usize;
      let mut c = indices[i * 3 + 2] as usize;

      if let Some(remap) = remap {
        a = remap[a] as usize;
        b = remap[b] as usize;
        c = remap[c] as usize;
      };

      let a = a as u32;
      let b = b as u32;
      let c = c as u32;

      self.data[self.offsets[a as usize] as usize] = HalfEdge { next: b, prev: c };
      self.data[self.offsets[b as usize] as usize] = HalfEdge { next: c, prev: a };
      self.data[self.offsets[c as usize] as usize] = HalfEdge { next: a, prev: b };

      self.offsets[a as usize] += 1;
      self.offsets[b as usize] += 1;
      self.offsets[c as usize] += 1;
    }

    // fix offsets that have been disturbed by the previous pass
    for (offset, count) in self.offsets.iter_mut().zip(self.counts.iter()) {
      assert!(*offset >= *count);

      *offset -= *count;
    }
  }

  pub fn has_edge(&self, a: u32, b: u32) -> bool {
    let count = self.counts[a as usize] as usize;
    let offset = self.offsets[a as usize] as usize;

    self.data[offset..offset + count]
      .iter()
      .any(|d| d.next == b)
  }
}
