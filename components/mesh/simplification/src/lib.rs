#![allow(clippy::disallowed_types)] // we have already used custom hasher
#![allow(clippy::too_many_arguments)]

use std::collections::{hash_map::Entry, HashMap};

use rendiation_algebra::*;
use rendiation_geometry::{Box3, Positioned};

mod qem;
use qem::*;

mod hasher;
use hasher::*;

mod vertex_kind;
use vertex_kind::*;

mod adjacency;
use adjacency::*;

const INVALID_INDEX: u32 = u32::MAX;

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
  let PositionalRemapping { remap, wedge } = build_position_remap(vertices);

  // classify vertices; vertex kind determines collapse rules, see `CAN_COLLAPSE`
  let ClassifyResult {
    vertex_kind,
    mut loop_,
    mut loopback,
  } = classify_vertices(vertices.len(), &adjacency, &remap, &wedge, lock_border);

  let vertex_positions = rescale_positions(vertices);

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
    adjacency.update(&result[0..result_count], Some(&remap));

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

/// rescale the vertex into unit cube with min(0,0,0)
fn rescale_positions<Vertex>(vertices: &[Vertex]) -> Vec<Vec3<f32>>
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let bbox: Box3 = vertices.iter().map(|v| v.position()).collect();
  let box_size = bbox.size();
  let extent = box_size.x.max(box_size.y).max(box_size.z);
  let scale = inversed_or_zeroed(extent);

  vertices
    .iter()
    .map(|v| (v.position() - bbox.min) * scale)
    .collect()
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

  nbc.dot(nbd) < 0.0
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

  for edge in adjacency.iter_vertex_outgoing_half_edges(i0) {
    let a = collapse_remap[edge.next as usize];
    let b = collapse_remap[edge.prev as usize];

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

struct PositionalRemapping {
  remap: Vec<u32>,
  wedge: Vec<u32>,
}

fn build_position_remap<Vertex>(vertices: &[Vertex]) -> PositionalRemapping
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  // build wedge table: for each vertex, which other vertex is the next wedge that also maps to the
  // same vertex? entries in table form a (cyclic) wedge loop per vertex; for manifold vertices,
  // wedge[i] == remap[i] == i
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

  PositionalRemapping { remap, wedge }
}

/// Generates vertex buffer from the source vertex buffer and remap table generated by
/// [generate_vertex_remap].
///
/// # Arguments
///
/// * `destination`: must contain enough space for the resulting vertex buffer
///   (`unique_vertex_count` elements, returned by [generate_vertex_remap])
/// * `vertices`: should have the initial vertex count and not the value returned by
///   [generate_vertex_remap]
pub fn remap_vertex_buffer<Vertex>(destination: &mut [Vertex], vertices: &[Vertex], remap: &[u32])
where
  Vertex: Copy,
{
  remap
    .iter()
    .filter(|dst| **dst != INVALID_INDEX)
    .enumerate()
    .for_each(|(src, dst)| destination[*dst as usize] = vertices[src]);
}

/// Generates a vertex remap table from the vertex buffer and an optional index buffer and returns
/// number of unique vertices.
///
/// As a result, all vertices that are binary equivalent map to the same (new) location, with no
/// gaps in the resulting sequence. Resulting remap table maps old vertices to new vertices and can
/// be used in [remap_vertex_buffer]/[remap_index_buffer].
///
/// Note that binary equivalence considers all `Stream::subset` bytes, including padding which
/// should be zero-initialized.
///
/// # Arguments
///
/// * `destination`: must contain enough space for the resulting remap table (`vertex_count`
///   elements defined by `vertices`)
/// * `indices`: can be `None` if the input is unindexed
pub fn generate_vertex_remap<Vertex: Eq + std::hash::Hash>(
  destination: &mut [u32],
  indices: Option<&[u32]>,
  vertices: &[Vertex],
) -> usize {
  generate_vertex_remap_inner(destination, indices, vertices.len(), |index| {
    vertices.get(index)
  })
}

fn generate_vertex_remap_inner<Vertex, Lookup>(
  destination: &mut [u32],
  indices: Option<&[u32]>,
  vertex_count: usize,
  lookup: Lookup,
) -> usize
where
  Lookup: Fn(usize) -> Vertex,
  Vertex: Eq + std::hash::Hash,
{
  let index_count = match indices {
    Some(buffer) => buffer.len(),
    None => vertex_count,
  };
  assert_eq!(index_count % 3, 0);

  destination.fill(INVALID_INDEX);

  let mut table = HashMap::with_capacity_and_hasher(vertex_count, BuildVertexHasher::default());

  let mut next_vertex = 0;

  for i in 0..index_count {
    let index = match indices {
      Some(buffer) => buffer[i] as usize,
      None => i,
    };
    assert!(index < vertex_count);

    if destination[index] == INVALID_INDEX {
      match table.entry(lookup(index)) {
        Entry::Occupied(entry) => {
          let value = *entry.get() as usize;
          assert!(destination[value] != INVALID_INDEX);
          destination[index] = destination[value];
        }
        Entry::Vacant(entry) => {
          entry.insert(index);
          destination[index] = next_vertex as u32;
          next_vertex += 1;
        }
      }
    }
  }

  assert!(next_vertex <= vertex_count);

  next_vertex
}
