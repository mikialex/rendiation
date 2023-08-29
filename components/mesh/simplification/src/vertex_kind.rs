use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub enum VertexKind {
  Manifold, // not on an attribute seam, not on any boundary
  Border,   // not on an attribute seam, has exactly two open edges
  Seam,     // on an attribute seam with exactly two attribute seam edges
  Complex,  /* none of the above; these vertices can move as long as all wedges move to the
             * target vertex */
  Locked, // none of the above; these vertices can't move
}

pub struct ClassifyResult {
  pub vertex_kind: Vec<VertexKind>,
  pub loop_: Vec<u32>,
  pub loopback: Vec<u32>,
}

pub fn classify_vertices(
  vertex_count: usize,
  adjacency: &EdgeAdjacency,
  remap: &[u32],
  wedge: &[u32],
  lock_border: bool,
) -> ClassifyResult {
  let mut result = vec![VertexKind::Manifold; vertex_count];

  // map vertex idx to the outcome half edge's end vertex idx;
  let mut loop_ = vec![INVALID_INDEX; vertex_count];
  // map vertex idx to the income half edge's start vertex idx;
  let mut loopback = vec![INVALID_INDEX; vertex_count];
  // the two mapping above:
  // if equals INVALID_INDEX, it's dangling line segment(I think it's impossible in our case?)
  // if equals self index, it's a none manifold vertex that shared with multiple in out half edge

  // incoming & outgoing open edges: `INVALID_INDEX` if no open edges, i if there are more than 1
  // note that this is the same data as required in loop[] arrays; loop[] data is only valid for
  // border/seam but here it's okay to fill the data out for other types of vertices as well
  let openinc = &mut loopback;
  let openout = &mut loop_;

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

  for i in 0..vertex_count {
    if remap[i] == i as u32 {
      if wedge[i] == i as u32 {
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
      } else if wedge[wedge[i] as usize] == i as u32 {
        // attribute seam; need to distinguish between Seam and Locked
        let w = wedge[i] as usize;
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
            result[i] = VertexKind::Seam;
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

  ClassifyResult {
    vertex_kind: result,
    loop_,
    loopback,
  }
}

impl VertexKind {
  pub fn index(&self) -> usize {
    match *self {
      VertexKind::Manifold => 0,
      VertexKind::Border => 1,
      VertexKind::Seam => 2,
      VertexKind::Complex => 3,
      VertexKind::Locked => 4,
    }
  }
}

pub const KIND_COUNT: usize = 5;

// manifold vertices can collapse onto anything
// border/seam vertices can only be collapsed onto border/seam respectively
// complex vertices can collapse onto complex/locked
// a rule of thumb is that collapsing kind A into kind B preserves the kind B in the target vertex
// for example, while we could collapse Complex into Manifold, this would mean the target vertex
// isn't Manifold anymore
pub const CAN_COLLAPSE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true, true, true, true, true],
  [false, true, false, false, false],
  [false, false, true, false, false],
  [false, false, false, true, true],
  [false, false, false, false, false],
];

// if a vertex is manifold or seam, adjoining edges are guaranteed to have an opposite edge
// note that for seam edges, the opposite edge isn't present in the attribute-based topology
// but is present if you consider a position-only mesh variant
pub const HAS_OPPOSITE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true, true, true, false, true],
  [true, false, true, false, false],
  [true, true, true, false, true],
  [false, false, false, false, false],
  [true, false, true, false, false],
];
