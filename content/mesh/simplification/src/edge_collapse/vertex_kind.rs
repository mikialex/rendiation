use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub enum VertexKind {
  Manifold,   // not on an attribute seam, not on any boundary
  Border,     // not on an attribute seam, has exactly two open edges
  SimpleSeam, // on an attribute seam with exactly two attribute seam edges
  // todo, not active used
  #[allow(dead_code)]
  Complex, /* none of the above; these vertices can move as long as all wedges move to the
            * target vertex */
  Locked, // none of the above; these vertices can't move
}

pub fn classify_vertices(
  adjacency: &EdgeAdjacency,
  borders: &BorderLoops,
  remap: &[u32],
  wedge: &VertexWedgeLoops,
  vertex_lock: Option<&[bool]>,
  lock_border: bool,
) -> Vec<VertexKind> {
  let vertex_count = adjacency.vertex_count();
  let mut result = vec![VertexKind::Manifold; vertex_count];

  for i in 0..vertex_count {
    if remap[i] == i as u32 {
      // no attribute seam, need to check if it's manifold
      if !wedge.vertex_is_on_seam(i) {
        let i_ = i as u32;

        if borders.vertex_has_no_edge(i_) {
          // note: we classify any vertices with no open edges as manifold
          // this is technically incorrect - if 4 triangles share an edge, we'll classify vertices as
          // manifold it's unclear if this is a problem in practice
          result[i] = VertexKind::Manifold;
        } else if borders.vertex_is_manifold(i_) {
          result[i] = VertexKind::Border;
        } else {
          result[i] = VertexKind::Locked;
        }
      } else if let Some(w) = wedge.vertex_is_on_simple_seam(i) {
        // attribute seam; need to distinguish between Seam and Locked
        let openiv = borders.get_half_edge_in_vertex(i as u32) as usize;
        let openov = borders.get_half_edge_out_vertex(i as u32) as usize;
        let openiw = borders.get_half_edge_in_vertex(w as u32) as usize;
        let openow = borders.get_half_edge_out_vertex(w as u32) as usize;

        // seam should have one open half-edge for each vertex, and the edges need to "connect" -
        // point to the same vertex post-remap
        if borders.vertex_is_border(i as u32) && borders.vertex_is_border(w as u32) {
          if remap[openiv] == remap[openow]
            && remap[openov] == remap[openiw]
            && remap[openiv] != remap[openov]
          {
            result[i] = VertexKind::SimpleSeam;
          } else {
            result[i] = VertexKind::Locked;
          }
        } else {
          result[i] = VertexKind::Locked;
        }
      } else {
        // more than one vertex maps to this one; we don't have classification available
        result[i] = VertexKind::Locked;
      }
    } else {
      assert!(remap[i] < i as u32);

      result[i] = result[remap[i] as usize];
    }
  }

  if let Some(vertex_lock) = vertex_lock {
    // vertex_lock may lock any wedge, not just the primary vertex, so we need to lock
    // the primary vertex and relock any wedges
    for i in 0..vertex_count {
      if vertex_lock[i] {
        result[remap[i] as usize] = VertexKind::Locked;
      }
    }

    for i in 0..vertex_count {
      if result[remap[i] as usize] == VertexKind::Locked {
        result[i] = VertexKind::Locked;
      }
    }
  }

  if lock_border {
    result.iter_mut().for_each(|v| {
      if let VertexKind::Border = v {
        *v = VertexKind::Locked
      }
    })
  }

  result
}

impl VertexKind {
  fn index(&self) -> usize {
    match *self {
      VertexKind::Manifold => 0,
      VertexKind::Border => 1,
      VertexKind::SimpleSeam => 2,
      VertexKind::Complex => 3,
      VertexKind::Locked => 4,
    }
  }
  pub fn has_opposite_edge(a: Self, b: Self) -> bool {
    HAS_OPPOSITE[a.index()][b.index()]
  }
  pub fn can_collapse_into(&self, other: Self) -> bool {
    CAN_COLLAPSE[self.index()][other.index()]
  }
}

const KIND_COUNT: usize = 5;

// manifold vertices can collapse onto anything
// border/seam vertices can collapse onto border/seam respectively, or locked
// complex vertices can collapse onto complex/locked
// a rule of thumb is that collapsing kind A into kind B preserves the kind B in the target vertex
// for example, while we could collapse Complex into Manifold, this would mean the target vertex
// isn't Manifold anymore
#[rustfmt::skip]
const CAN_COLLAPSE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true,  true,  true,  true,  true ],
  [false, true,  false, false, true],
  [false, false, true,  false, true],
  [false, false, false, true,  true ],
  [false, false, false, false, false],
];

// if a vertex is manifold or seam, adjoining edges are guaranteed to have an opposite edge
// note that for seam edges, the opposite edge isn't present in the attribute-based topology
// but is present if you consider a position-only mesh variant
#[rustfmt::skip]
const HAS_OPPOSITE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true,  true,  true,  false, true ],
  [true,  false, true,  false, false],
  [true,  true,  true,  false, true ],
  [false, false, false, false, false],
  [true,  false, true,  false, false],
];
