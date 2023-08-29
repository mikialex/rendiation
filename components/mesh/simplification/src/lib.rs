#![allow(clippy::disallowed_types)] // we have already used custom hasher
#![allow(clippy::too_many_arguments)]

use std::collections::{hash_map::Entry, HashMap};

use rendiation_algebra::*;
use rendiation_geometry::Positioned;

mod qem;
use qem::*;

mod hasher;
use hasher::*;

mod adjacency;
use adjacency::*;

pub const INVALID_INDEX: u32 = u32::MAX;

/// Reduces the number of triangles in the mesh, attempting to preserve mesh appearance as much as
/// possible.
///
/// The algorithm tries to preserve mesh topology and can stop short of the target goal based on
/// topology constraints or target error. If not all attributes from the input mesh are required,
/// it's recommended to reindex the mesh  prior to simplification.
///
/// Returns the number of indices after simplification, with destination containing new index data.
/// The resulting index buffer references vertices from the original vertex buffer.
/// If the original vertex data isn't required, creating a compact vertex buffer is recommended.
///
/// # Arguments
///
/// * `destination`: must contain enough space for the **source** index buffer (since optimization
///   is iterative, this means `indices.len()` elements - **not** `target_index_count`!)
pub fn simplify<Vertex>(
  destination: &mut [u32],
  indices: &[u32],
  vertices: &[Vertex],
  target_index_count: usize,
  target_error: f32,
  lock_border: bool,
) -> (usize, f32)
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  assert_eq!(indices.len() % 3, 0);
  assert!(target_index_count <= indices.len());

  let result = &mut destination[0..indices.len()];

  // build adjacency information
  let mut adjacency = EdgeAdjacency::new(indices, vertices.len());

  // build position remap that maps each vertex to the one with identical position
  let mut remap = vec![0u32; vertices.len()];
  let mut wedge = vec![0u32; vertices.len()];
  build_position_remap(&mut remap, &mut wedge, vertices);

  // classify vertices; vertex kind determines collapse rules, see `CAN_COLLAPSE`
  let mut vertex_kind = vec![VertexKind::Manifold; vertices.len()];
  let mut loop_ = vec![INVALID_INDEX; vertices.len()];
  let mut loopback = vec![INVALID_INDEX; vertices.len()];
  classify_vertices(
    &mut vertex_kind,
    &mut loop_,
    &mut loopback,
    vertices.len(),
    &adjacency,
    &remap,
    &wedge,
    lock_border,
  );

  let mut vertex_positions = vec![Vec3::default(); vertices.len()]; // TODO: spare init?
  rescale_positions(&mut vertex_positions, vertices);

  let mut vertex_quadrics = vec![Quadric::default(); vertices.len()];
  fill_face_quadrics(&mut vertex_quadrics, indices, &vertex_positions, &remap);
  fill_edge_quadrics(
    &mut vertex_quadrics,
    indices,
    &vertex_positions,
    &remap,
    &vertex_kind,
    &loop_,
    &loopback,
  );

  result.copy_from_slice(indices);

  // TODO: skip init?
  let mut edge_collapses = vec![Collapse::default(); indices.len()];
  let mut collapse_order = vec![0u32; indices.len()];
  let mut collapse_remap = vec![0u32; vertices.len()];

  let mut collapse_locked = vec![false; vertices.len()];

  let mut result_count = indices.len();
  let mut result_error = 0.;

  // `target_error` input is linear; we need to adjust it to match `Quadric::error` units
  let error_limit = target_error * target_error;

  while result_count > target_index_count {
    // note: throughout the simplification process adjacency structure reflects welded topology for
    // result-in-progress
    adjacency.update(result.get(0..result_count).unwrap(), Some(&remap));

    let edge_collapse_count = pick_edge_collapses(
      &mut edge_collapses,
      &result[0..result_count],
      &remap,
      &vertex_kind,
      &loop_,
    );

    // no edges can be collapsed any more due to topology restrictions
    if edge_collapse_count == 0 {
      break;
    }

    rank_edge_collapses(
      &mut edge_collapses[0..edge_collapse_count],
      &vertex_positions,
      &vertex_quadrics,
      &remap,
    );

    sort_edge_collapses(&mut collapse_order, &edge_collapses[0..edge_collapse_count]);

    let triangle_collapse_goal = (result_count - target_index_count) / 3;

    for (i, r) in collapse_remap.iter_mut().enumerate() {
      *r = i as u32;
    }

    collapse_locked.fill(false);

    let collapses = perform_edge_collapses(
      &mut collapse_remap,
      &mut collapse_locked,
      &mut vertex_quadrics,
      &edge_collapses[0..edge_collapse_count],
      &collapse_order,
      &remap,
      &wedge,
      &vertex_kind,
      &vertex_positions,
      &adjacency,
      triangle_collapse_goal,
      error_limit,
      &mut result_error,
    );

    // no edges can be collapsed any more due to hitting the error limit or triangle collapse limit
    if collapses == 0 {
      break;
    }

    remap_edge_loops(&mut loop_, &collapse_remap);
    remap_edge_loops(&mut loopback, &collapse_remap);

    let new_count = remap_index_buffer(&mut result[0..result_count], &collapse_remap);
    assert!(new_count < result_count);

    result_count = new_count;
  }

  // result_error is quadratic; we need to remap it back to linear
  let out_result_error = result_error.sqrt();

  (result_count, out_result_error)
}

pub fn calc_pos_extents<Vertex>(vertices: &[Vertex]) -> ([f32; 3], f32)
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let mut minv = [f32::MAX; 3];
  let mut maxv = [-f32::MAX; 3];

  for vertex in vertices {
    let v = vertex.position();

    for j in 0..3 {
      minv[j] = minv[j].min(v[j]);
      maxv[j] = maxv[j].max(v[j]);
    }
  }

  let extent = (maxv[0] - minv[0])
    .max(maxv[1] - minv[1])
    .max(maxv[2] - minv[2]);

  (minv, extent)
}

fn rescale_positions<Vertex>(result: &mut [Vec3<f32>], vertices: &[Vertex])
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let (minv, extent) = calc_pos_extents(vertices);

  for (i, vertex) in vertices.iter().enumerate() {
    result[i] = vertex.position();
  }

  let scale = inversed_or_zeroed(extent);

  for pos in result {
    pos.x = (pos.x - minv[0]) * scale;
    pos.y = (pos.y - minv[1]) * scale;
    pos.z = (pos.z - minv[2]) * scale;
  }
}

#[derive(Clone, Default)]
struct Collapse {
  v0: u32,
  v1: u32,
  u: CollapseUnion,
}

union CollapseUnion {
  bidi: u32,
  error: f32,
  errorui: u32,
}

impl Clone for CollapseUnion {
  fn clone(&self) -> Self {
    Self {
      bidi: unsafe { self.bidi },
    }
  }
}

impl Default for CollapseUnion {
  fn default() -> Self {
    Self { bidi: 0 }
  }
}

fn pick_edge_collapses(
  collapses: &mut [Collapse],
  indices: &[u32],
  remap: &[u32],
  vertex_kind: &[VertexKind],
  loop_: &[u32],
) -> usize {
  let mut collapse_count = 0;

  for i in indices.chunks_exact(3) {
    const NEXT: [usize; 3] = [1, 2, 0];

    for e in 0..3 {
      let i0 = i[e] as usize;
      let i1 = i[NEXT[e]] as usize;

      // this can happen either when input has a zero-length edge, or when we perform collapses for
      // complex topology w/seams and collapse a manifold vertex that connects to both wedges
      // onto one of them we leave edges like this alone since they may be important for
      // preserving mesh integrity
      if remap[i0] == remap[i1] {
        continue;
      }

      let k0 = vertex_kind[i0];
      let k1 = vertex_kind[i1];

      // the edge has to be collapsible in at least one direction
      if !(CAN_COLLAPSE[k0.index()][k1.index()] || CAN_COLLAPSE[k1.index()][k0.index()]) {
        continue;
      }

      // manifold and seam edges should occur twice (i0->i1 and i1->i0) - skip redundant edges
      if HAS_OPPOSITE[k0.index()][k1.index()] && remap[i1] > remap[i0] {
        continue;
      }

      // two vertices are on a border or a seam, but there's no direct edge between them
      // this indicates that they belong to two different edge loops and we should not collapse this
      // edge loop[] tracks half edges so we only need to check i0->i1
      if k0 == k1 && (k0 == VertexKind::Border || k0 == VertexKind::Seam) && loop_[i0] != i1 as u32
      {
        continue;
      }

      // edge can be collapsed in either direction - we will pick the one with minimum error
      // note: we evaluate error later during collapse ranking, here we just tag the edge as
      // bidirectional
      if CAN_COLLAPSE[k0.index()][k1.index()] & CAN_COLLAPSE[k1.index()][k0.index()] {
        let c = Collapse {
          v0: i0 as u32,
          v1: i1 as u32,
          u: CollapseUnion { bidi: 1 },
        };
        collapses[collapse_count] = c;
        collapse_count += 1;
      } else {
        // edge can only be collapsed in one direction
        let e0 = if CAN_COLLAPSE[k0.index()][k1.index()] {
          i0
        } else {
          i1
        };
        let e1 = if CAN_COLLAPSE[k0.index()][k1.index()] {
          i1
        } else {
          i0
        };

        let c = Collapse {
          v0: e0 as u32,
          v1: e1 as u32,
          u: CollapseUnion { bidi: 0 },
        };
        collapses[collapse_count] = c;
        collapse_count += 1;
      }
    }
  }

  collapse_count
}

fn rank_edge_collapses(
  collapses: &mut [Collapse],
  vertex_positions: &[Vec3<f32>],
  vertex_quadrics: &[Quadric],
  remap: &[u32],
) {
  for c in collapses {
    let i0 = c.v0;
    let i1 = c.v1;

    // most edges are bidirectional which means we need to evaluate errors for two collapses
    // to keep this code branchless we just use the same edge for unidirectional edges
    let j0 = unsafe {
      if c.u.bidi != 0 {
        i1
      } else {
        i0
      }
    };
    let j1 = unsafe {
      if c.u.bidi != 0 {
        i0
      } else {
        i1
      }
    };

    let qi = vertex_quadrics[remap[i0 as usize] as usize];
    let qj = vertex_quadrics[remap[j0 as usize] as usize];

    let ei = qi.error(&vertex_positions[i1 as usize]);
    let ej = qj.error(&vertex_positions[j1 as usize]);

    // pick edge direction with minimal error
    c.v0 = if ei <= ej { i0 } else { j0 };
    c.v1 = if ei <= ej { i1 } else { j1 };
    c.u.error = ei.min(ej);
  }
}

#[allow(clippy::needless_range_loop)]
fn sort_edge_collapses(sort_order: &mut [u32], collapses: &[Collapse]) {
  const SORT_BITS: usize = 11;

  // fill histogram for counting sort
  let mut histogram = [0u32; 1 << SORT_BITS];

  for c in collapses {
    // skip sign bit since error is non-negative
    let key = unsafe { (c.u.errorui << 1) >> (32 - SORT_BITS) };

    histogram[key as usize] += 1;
  }

  // compute offsets based on histogram data
  let mut histogram_sum = 0;

  for i in 0..(1 << SORT_BITS) {
    let count = histogram[i];
    histogram[i] = histogram_sum;
    histogram_sum += count;
  }

  assert_eq!(histogram_sum as usize, collapses.len());

  // compute sort order based on offsets
  for (i, c) in collapses.iter().enumerate() {
    // skip sign bit since error is non-negative
    let key = unsafe { ((c.u.errorui << 1) >> (32 - SORT_BITS)) as usize };

    sort_order[histogram[key] as usize] = i as u32;
    histogram[key] += 1;
  }
}

fn perform_edge_collapses(
  collapse_remap: &mut [u32],
  collapse_locked: &mut [bool],
  vertex_quadrics: &mut [Quadric],
  collapses: &[Collapse],
  collapse_order: &[u32],
  remap: &[u32],
  wedge: &[u32],
  vertex_kind: &[VertexKind],
  vertex_positions: &[Vec3<f32>],
  adjacency: &EdgeAdjacency,
  triangle_collapse_goal: usize,
  error_limit: f32,
  result_error: &mut f32,
) -> usize {
  let collapse_count = collapses.len();
  let mut edge_collapses = 0;
  let mut triangle_collapses = 0;

  // most collapses remove 2 triangles; use this to establish a bound on the pass in terms of error
  // limit note that edge_collapse_goal is an estimate; triangle_collapse_goal will be used to
  // actually limit collapses
  let mut edge_collapse_goal = triangle_collapse_goal / 2;

  for order in collapse_order {
    let c = collapses[*order as usize].clone();

    let error = unsafe { c.u.error };

    if error > error_limit {
      break;
    }

    if triangle_collapses >= triangle_collapse_goal {
      break;
    }

    // we limit the error in each pass based on the error of optimal last collapse; since many
    // collapses will be locked as they will share vertices with other successfull collapses, we
    // need to increase the acceptable error by some factor
    let error_goal = if edge_collapse_goal < collapse_count {
      let c_ = &collapses[collapse_order[edge_collapse_goal] as usize];
      1.5 * unsafe { c_.u.error }
    } else {
      f32::MAX
    };

    // on average, each collapse is expected to lock 6 other collapses; to avoid degenerate passes
    // on meshes with odd topology, we only abort if we got over 1/6 collapses accordingly.
    if error > error_goal && triangle_collapses > triangle_collapse_goal / 6 {
      break;
    }

    let i0 = c.v0 as usize;
    let i1 = c.v1 as usize;

    let r0 = remap[i0] as usize;
    let r1 = remap[i1] as usize;

    // we don't collapse vertices that had source or target vertex involved in a collapse
    // it's important to not move the vertices twice since it complicates the tracking/remapping
    // logic it's important to not move other vertices towards a moved vertex to preserve error
    // since we don't re-rank collapses mid-pass
    if collapse_locked[r0] || collapse_locked[r1] {
      continue;
    }

    if has_triangle_flips(adjacency, vertex_positions, collapse_remap, r0, r1) {
      // adjust collapse goal since this collapse is invalid and shouldn't factor into error goal
      edge_collapse_goal += 1;

      continue;
    }

    assert_eq!(collapse_remap[r0] as usize, r0);
    assert_eq!(collapse_remap[r1] as usize, r1);

    vertex_quadrics[r1] += vertex_quadrics[r0];

    match vertex_kind[i0] {
      VertexKind::Complex => {
        let mut v = i0;

        loop {
          collapse_remap[v] = r1 as u32;
          v = wedge[v] as usize;

          if v == i0 {
            break;
          }
        }
      }
      VertexKind::Seam => {
        // remap v0 to v1 and seam pair of v0 to seam pair of v1
        let s0 = wedge[i0] as usize;
        let s1 = wedge[i1] as usize;

        assert!(s0 != i0 && s1 != i1);
        assert!(wedge[s0] as usize == i0 && wedge[s1] as usize == i1);

        collapse_remap[i0] = i1 as u32;
        collapse_remap[s0] = s1 as u32;
      }
      _ => {
        assert_eq!(wedge[i0] as usize, i0);

        collapse_remap[i0] = i1 as u32;
      }
    }

    collapse_locked[r0] = true;
    collapse_locked[r1] = true;

    // border edges collapse 1 triangle, other edges collapse 2 or more
    triangle_collapses += if vertex_kind[i0] == VertexKind::Border {
      1
    } else {
      2
    };
    edge_collapses += 1;

    *result_error = if *result_error < error {
      error
    } else {
      *result_error
    };
  }

  edge_collapses
}

// does triangle ABC flip when C is replaced with D?
fn has_triangle_flip(a: Vec3<f32>, b: Vec3<f32>, c: Vec3<f32>, d: Vec3<f32>) -> bool {
  let eb = b - a;
  let ec = c - a;
  let ed = d - a;

  let nbc = eb.cross(ec);
  let nbd = eb.cross(ed);

  nbc.x * nbd.x + nbc.y * nbd.y + nbc.z * nbd.z < 0.0
}

fn has_triangle_flips(
  adjacency: &EdgeAdjacency,
  vertex_positions: &[Vec3<f32>],
  collapse_remap: &[u32],
  i0: usize,
  i1: usize,
) -> bool {
  assert_eq!(collapse_remap[i0] as usize, i0);
  assert_eq!(collapse_remap[i1] as usize, i1);

  let v0 = vertex_positions[i0];
  let v1 = vertex_positions[i1];

  let count = adjacency.counts[i0] as usize;
  let edges = &adjacency.data[adjacency.offsets[i0] as usize..count];

  for i in 0..count {
    let a = collapse_remap[edges[i].next as usize];
    let b = collapse_remap[edges[i].prev as usize];

    // skip triangles that get collapsed
    // note: this is mathematically redundant as if either of these is true, the dot product in
    // hasTriangleFlip should be 0
    if a == i1 as u32 || b == i1 as u32 {
      continue;
    }

    // early-out when at least one triangle flips due to a collapse
    if has_triangle_flip(
      vertex_positions[a as usize],
      vertex_positions[b as usize],
      v0,
      v1,
    ) {
      return true;
    }
  }

  false
}

fn remap_index_buffer(indices: &mut [u32], collapse_remap: &[u32]) -> usize {
  let mut write = 0;

  for i in (0..indices.len()).step_by(3) {
    let v0 = collapse_remap[indices[i] as usize];
    let v1 = collapse_remap[indices[i + 1] as usize];
    let v2 = collapse_remap[indices[i + 2] as usize];

    // we never move the vertex twice during a single pass
    assert_eq!(collapse_remap[v0 as usize], v0);
    assert_eq!(collapse_remap[v1 as usize], v1);
    assert_eq!(collapse_remap[v2 as usize], v2);

    if v0 != v1 && v0 != v2 && v1 != v2 {
      indices[write] = v0;
      indices[write + 1] = v1;
      indices[write + 2] = v2;
      write += 3;
    }
  }

  write
}

fn remap_edge_loops(loop_: &mut [u32], collapse_remap: &[u32]) {
  for i in 0..loop_.len() {
    if loop_[i] != INVALID_INDEX {
      let l = loop_[i];
      let r = collapse_remap[l as usize];

      // i == r is a special case when the seam edge is collapsed in a direction opposite to where
      // loop goes
      loop_[i] = if i == r as usize {
        loop_[l as usize]
      } else {
        r
      };
    }
  }
}

fn build_position_remap<Vertex>(remap: &mut [u32], wedge: &mut [u32], vertices: &[Vertex])
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let mut table = HashMap::with_capacity_and_hasher(vertices.len(), BuildPositionHasher::default());

  // build forward remap: for each vertex, which other (canonical) vertex does it map to?
  // we use position equivalence for this, and remap vertices to other existing vertices
  for (index, vertex) in vertices.iter().enumerate() {
    remap[index] = match table.entry(VertexPosition(vertex.position().into())) {
      Entry::Occupied(entry) => *entry.get(),
      Entry::Vacant(entry) => {
        entry.insert(index as u32);
        index as u32
      }
    };
  }

  // build wedge table: for each vertex, which other vertex is the next wedge that also maps to the
  // same vertex? entries in table form a (cyclic) wedge loop per vertex; for manifold vertices,
  // wedge[i] == remap[i] == i
  for (i, w) in wedge.iter_mut().enumerate() {
    *w = i as u32;
  }

  for (i, ri) in remap.iter().enumerate() {
    let ri = *ri as usize;

    if ri != i {
      let r = ri;

      wedge[i] = wedge[r];
      wedge[r] = i as u32;
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
pub enum VertexKind {
  Manifold, // not on an attribute seam, not on any boundary
  Border,   // not on an attribute seam, has exactly two open edges
  Seam,     // on an attribute seam with exactly two attribute seam edges
  Complex,  /* none of the above; these vertices can move as long as all wedges move to the
             * target vertex */
  Locked, // none of the above; these vertices can't move
}

fn classify_vertices(
  result: &mut [VertexKind],
  loop_: &mut [u32],
  loopback: &mut [u32],
  vertex_count: usize,
  adjacency: &EdgeAdjacency,
  remap: &[u32],
  wedge: &[u32],
  lock_border: bool,
) {
  // incoming & outgoing open edges: `INVALID_INDEX` if no open edges, i if there are more than 1
  // note that this is the same data as required in loop[] arrays; loop[] data is only valid for
  // border/seam but here it's okay to fill the data out for other types of vertices as well
  let openinc = loopback;
  let openout = loop_;

  for vertex in 0..vertex_count {
    let offset = adjacency.offsets[vertex] as usize;
    let count = adjacency.counts[vertex] as usize;

    let edges = &adjacency.data[offset..offset + count];

    for edge in edges {
      let target = edge.next;

      if target == vertex as u32 {
        // degenerate triangles have two distinct edges instead of three, and the self edge
        // is bi-directional by definition; this can break border/seam classification by "closing"
        // the open edge from another triangle and falsely marking the vertex as manifold
        // instead we mark the vertex as having >1 open edges which turns it into locked/complex
        openinc[vertex] = vertex as u32;
        openout[vertex] = vertex as u32;
      } else if !adjacency.has_edge(target, vertex as u32) {
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

const KIND_COUNT: usize = 5;

// manifold vertices can collapse onto anything
// border/seam vertices can only be collapsed onto border/seam respectively
// complex vertices can collapse onto complex/locked
// a rule of thumb is that collapsing kind A into kind B preserves the kind B in the target vertex
// for example, while we could collapse Complex into Manifold, this would mean the target vertex
// isn't Manifold anymore
const CAN_COLLAPSE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true, true, true, true, true],
  [false, true, false, false, false],
  [false, false, true, false, false],
  [false, false, false, true, true],
  [false, false, false, false, false],
];

// if a vertex is manifold or seam, adjoining edges are guaranteed to have an opposite edge
// note that for seam edges, the opposite edge isn't present in the attribute-based topology
// but is present if you consider a position-only mesh variant
const HAS_OPPOSITE: [[bool; KIND_COUNT]; KIND_COUNT] = [
  [true, true, true, false, true],
  [true, false, true, false, false],
  [true, true, true, false, true],
  [false, false, false, false, false],
  [true, false, true, false, false],
];
