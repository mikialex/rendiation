use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub enum VertexKind {
  Manifold,   // not on an attribute seam, not on any boundary
  Border,     // not on an attribute seam, has exactly two open edges
  SimpleSeam, // on an attribute seam with exactly two attribute seam edges
  // todo, check why not active used
  #[allow(dead_code)]
  Complex, /* none of the above; these vertices can move as long as all wedges move to the
            * target vertex */
  Locked, // none of the above; these vertices can't move
}

pub fn classify_vertices(
  adjacency: &EdgeAdjacency,
  BorderLoops { openout, openinc }: &BorderLoops,
  remap: &[u32],
  wedge: &VertexWedgeLoops,
  lock_border: bool,
) -> Vec<VertexKind> {
  let vertex_count = adjacency.vertex_count();
  let mut result = vec![VertexKind::Manifold; vertex_count];

  for i in 0..vertex_count {
    if remap[i] == i as u32 {
      if !wedge.vertex_is_on_seam(i) {
        // no attribute seam, need to check if it's manifold
        let openi = openinc[i];
        let openo = openout[i];

        // note: we classify any vertices with no open edges as manifold
        // this is technically incorrect - if 4 triangles share an edge, we'll classify vertices as
        // manifold it's unclear if this is a problem in practice
        if openi == INVALID_INDEX && openo == INVALID_INDEX {
          result[i] = VertexKind::Manifold;
        } else if openi != i as u32 && openo != i as u32 {
          result[i] = VertexKind::Border;
        } else {
          result[i] = VertexKind::Locked;
        }
      } else if let Some(w) = wedge.vertex_is_on_simple_seam(i) {
        // attribute seam; need to distinguish between Seam and Locked
        let openiv = openinc[i] as usize;
        let openov = openout[i] as usize;
        let openiw = openinc[w] as usize;
        let openow = openout[w] as usize;

        // seam should have one open half-edge for each vertex, and the edges need to "connect" -
        // point to the same vertex post-remap
        if openiv != INVALID_INDEX as usize
          && openiv != i
          && openov != INVALID_INDEX as usize
          && openov != i
          && openiw != INVALID_INDEX as usize
          && openiw != w
          && openow != INVALID_INDEX as usize
          && openow != w
        {
          if remap[openiv] == remap[openow] && remap[openov] == remap[openiw] {
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
  pub fn has_opposite(a: Self, b: Self) -> bool {
    HAS_OPPOSITE[a.index()][b.index()]
  }
  pub fn can_collapse(a: Self, b: Self) -> bool {
    CAN_COLLAPSE[a.index()][b.index()]
  }
}

const KIND_COUNT: usize = 5;

// manifold vertices can collapse onto anything
// border/seam vertices can only be collapsed onto border/seam respectively
// complex vertices can collapse onto complex/locked
// a rule of thumb is that collapsing kind A into kind B preserves the kind B in the target vertex
// for example, while we could collapse Complex into Manifold, this would mean the target vertex
// isn't Manifold anymore
#[rustfmt::skip]
const CAN_COLLAPSE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true,  true,  true,  true,  true ],
  [false, true,  false, false, false],
  [false, false, true,  false, false],
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
