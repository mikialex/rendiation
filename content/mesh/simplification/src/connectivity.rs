use crate::*;

pub struct EdgeAdjacency {
  counts: Vec<u32>,
  offsets: Vec<u32>,
  data: Vec<HalfEdge>,
}

#[derive(Default, Clone, Copy)]
pub struct HalfEdge {
  pub next: u32,
  pub prev: u32,
}

impl EdgeAdjacency {
  pub fn new(indices: &[u32], vertex_count: usize) -> Self {
    let mut result = Self {
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

  pub fn vertex_count(&self) -> usize {
    self.counts.len()
  }

  pub fn has_half_edge(&self, from: u32, to: u32) -> bool {
    let count = self.counts[from as usize] as usize;
    let offset = self.offsets[from as usize] as usize;

    self.data[offset..offset + count]
      .iter()
      .any(|d| d.next == to)
  }

  pub fn iter_vertex_outgoing_half_edges(&self, v: usize) -> impl Iterator<Item = &HalfEdge> {
    let offset = self.offsets[v] as usize;
    let count = self.counts[v] as usize;

    self.data[offset..offset + count].iter()
  }
}

/// About mapping:
/// if equals INVALID_INDEX, it's no open edges
/// if equals self index, it's a none manifold vertex that shared with multiple in out half edge
pub struct BorderLoops {
  /// map vertex idx to it's out target vertex idx;
  openout: Vec<u32>,
  /// map vertex idx to it's source target vertex idx;
  openinc: Vec<u32>,
}

impl BorderLoops {
  pub fn get_half_edge_out_vertex(&self, vertex: u32) -> u32 {
    self.openout[vertex as usize]
  }

  pub fn get_half_edge_in_vertex(&self, vertex: u32) -> u32 {
    self.openinc[vertex as usize]
  }

  pub fn vertex_has_no_edge(&self, vertex: u32) -> bool {
    self.openout[vertex as usize] == INVALID_INDEX && self.openinc[vertex as usize] == INVALID_INDEX
  }

  pub fn vertex_is_border(&self, vertex: u32) -> bool {
    self.openout[vertex as usize] != INVALID_INDEX
      && self.openinc[vertex as usize] != INVALID_INDEX
      && self.vertex_is_manifold(vertex)
  }

  pub fn vertex_is_manifold(&self, vertex: u32) -> bool {
    self.openout[vertex as usize] != vertex && self.openinc[vertex as usize] != vertex
  }

  pub fn new(adjacency: &EdgeAdjacency) -> Self {
    let vertex_count = adjacency.vertex_count();
    let mut openout = vec![INVALID_INDEX; vertex_count];
    let mut openinc = vec![INVALID_INDEX; vertex_count];

    // loop[] data is only valid for border but here it's okay to fill the data out for other
    // types of vertices as well

    for vertex in 0..vertex_count {
      for edge in adjacency.iter_vertex_outgoing_half_edges(vertex) {
        let target = edge.next;

        if target == vertex as u32 {
          // degenerate triangles have two distinct edges instead of three, and the self edge
          // is bi-directional by definition; this can break border/seam classification by "closing"
          // the open edge from another triangle and falsely marking the vertex as manifold
          // instead we mark the vertex as having >1 open edges which turns it into locked/complex
          openinc[vertex] = vertex as u32;
          openout[vertex] = vertex as u32;
        } else if !adjacency.has_half_edge(target, vertex as u32) {
          openinc[target as usize] = if openinc[target as usize] == INVALID_INDEX {
            vertex as u32
          } else {
            target
          };
          openout[vertex] = if openout[vertex] == INVALID_INDEX {
            target
          } else {
            vertex as u32
          };
        }
      }
    }

    Self { openout, openinc }
  }

  pub fn remap_edge_loops(&mut self, collapse_remap: &[u32]) {
    remap_edge_loops(&mut self.openout, collapse_remap);
    remap_edge_loops(&mut self.openinc, collapse_remap);
  }
}

fn remap_edge_loops(loop_: &mut [u32], collapse_remap: &[u32]) {
  for i in 0..loop_.len() {
    // note: this is a no-op for vertices that were remapped
    // ideally we would clear the loop entries for those for consistency, even though they aren't going to be used
    // however, the remapping process needs loop information for remapped vertices, so this would require a separate pass
    if loop_[i] != INVALID_INDEX {
      let l = loop_[i];
      let r = collapse_remap[l as usize];

      // i == r is a special case when the seam edge is collapsed in a direction opposite to where
      // loop goes
      loop_[i] = if i == r as usize {
        let v = loop_[l as usize];
        if v != INVALID_INDEX {
          collapse_remap[v as usize]
        } else {
          INVALID_INDEX
        }
      } else {
        r
      };
    }
  }
}
