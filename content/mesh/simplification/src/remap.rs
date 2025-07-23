use crate::*;

pub struct PositionalRemapping {
  pub remap: Vec<u32>,
  pub wedge: VertexWedgeLoops,
}

/// for each vertex, which other vertex is the next wedge that also maps to the
/// same vertex? entries in table form a (cyclic) wedge loop per vertex; for manifold vertices,
/// wedge[i] == remap[i] == i
pub struct VertexWedgeLoops {
  wedge: Vec<u32>,
}

impl VertexWedgeLoops {
  pub fn vertex_is_on_seam(&self, vertex_id: usize) -> bool {
    self.wedge[vertex_id] != vertex_id as u32
  }
  /// the simple seam is that only two vertex with same position form this seam
  /// if it's simple seam, return the pair vertex id, if not, return None
  pub fn vertex_is_on_simple_seam(&self, vertex_id: usize) -> Option<usize> {
    let next = self.wedge[vertex_id] as usize;
    let back = self.wedge[next] as usize;
    (back == vertex_id).then_some(next)
  }
  pub fn next_same_position_vertex(&self, vertex_id: usize) -> u32 {
    self.wedge[vertex_id]
  }
}

pub fn build_position_remap<Vertex>(vertices: &[Vertex]) -> PositionalRemapping
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let mut wedge: Vec<_> = (0..vertices.len() as u32).collect();

  // for example we have (position, other attribute)
  // [(a, x), (a, x), (b, x), (c, x), (c, x), (d, x), (c, x)]
  // we have remap:
  // [0, 0, 2, 3, 3, 5, 3]
  // we have wedge:
  // [1, 0, 2, 6, 3, 5, 4]
  // in wedge table we can see the loop around vertex c: 3 -> 6 -> 4 -> 3
  // the "loop" is just a loop link list to visit all vertex that share one same position, do not
  // have the geometric meaning.

  let mut table = HashMap::with_capacity_and_hasher(vertices.len(), BuildPositionHasher::default());

  // build forward remap: for each vertex, which other (canonical) vertex does it map to?
  // we use position equivalence for this, and remap vertices to other existing vertices
  let remap: Vec<_> = vertices
    .iter()
    .enumerate()
    .map(
      |(i, vertex)| match table.entry(VertexPosition(vertex.position().into())) {
        Entry::Occupied(entry) => {
          let ri = *entry.get();

          let r = ri as usize;
          if r != i {
            wedge[i] = wedge[r];
            wedge[r] = i as u32;
          }

          ri
        }
        Entry::Vacant(entry) => {
          entry.insert(i as u32);
          i as u32
        }
      },
    )
    .collect();

  let wedge = VertexWedgeLoops { wedge };
  PositionalRemapping { remap, wedge }
}
