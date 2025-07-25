use crate::*;

mod vertex_kind;
pub use vertex_kind::*;

#[derive(Clone, Copy, Debug)]
pub struct EdgeCollapseConfig {
  /// the target index count to simplify, it may be not achieved due to the topology constraint and
  /// error config
  pub target_index_count: usize,
  /// the max error rate allowed in simplify.
  pub target_error: f32,
  pub use_absolute_error: bool,
  /// if the border allow to be simplify. User provide vertex lock will still worked as supplement
  pub lock_border: bool,
}

#[derive(Clone, Copy)]
pub struct EdgeCollapseResult {
  /// the result error rate
  pub result_error: f32,
  /// the number of indices after simplification.
  ///
  ///  The resulting index buffer references vertices from the original vertex buffer.
  /// If the original vertex data isn't required, creating a compact vertex buffer is recommended.
  pub result_count: usize,
}

/// Reduces the number of triangles in the mesh, attempting to preserve mesh appearance as much as
/// possible.
///
/// The algorithm tries to preserve mesh topology and can stop short of the target goal based on
/// topology constraints or target error. If not all attributes from the input mesh are required,
/// it's recommended to reindex the mesh  prior to simplification.
///
/// ## Arguments
///
/// * `destination`: must contain enough space for the **source** index buffer
pub fn simplify_by_edge_collapse<V>(
  destination: &mut [u32],
  indices: &[u32],
  vertices: &[V],
  vertex_lock: Option<&[bool]>,
  EdgeCollapseConfig {
    target_index_count,
    target_error,
    lock_border,
    use_absolute_error,
  }: EdgeCollapseConfig,
) -> EdgeCollapseResult
where
  V: Positioned<Position = Vec3<f32>>,
{
  assert_eq!(indices.len() % 3, 0);
  assert!(target_error >= 0.);
  assert!(target_index_count <= indices.len());

  let result = &mut destination[0..indices.len()];

  // build connectivity information
  let mut adjacency = EdgeAdjacency::new(indices, vertices.len());
  let mut border_loop = BorderLoops::new(&adjacency);

  // build position remap that maps each vertex to the one with identical position
  let PositionalRemapping { remap, wedge } = build_position_remap(vertices);

  // classify vertices; vertex kind determines collapse rules, see `CAN_COLLAPSE`
  let vertex_kind = classify_vertices(
    &adjacency,
    &border_loop,
    &remap,
    &wedge,
    vertex_lock,
    lock_border,
  );

  let (vertex_scale, vertex_positions) = rescale_positions(vertices);

  // todo!(); // fill other quadrics
  let mut vertex_quadrics = fill_edge_quadrics(
    indices,
    &vertex_positions,
    &remap,
    &vertex_kind,
    &border_loop,
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
  let error_scale = if use_absolute_error { vertex_scale } else { 1. };
  let error_limit = (target_error * target_error) / (error_scale * error_scale);

  while result_count > target_index_count {
    // note: throughout the simplification process adjacency structure reflects welded topology for
    // result-in-progress
    adjacency.update(&result[0..result_count], Some(&remap));

    let edge_collapse_count = pick_edge_collapses(
      &mut edge_collapses,
      &result[0..result_count],
      &vertex_positions,
      &vertex_quadrics,
      &remap,
      &vertex_kind,
      &border_loop,
    );

    // no edges can be collapsed any more due to topology restrictions
    if edge_collapse_count == 0 {
      break;
    }

    sort_edge_collapses(&mut collapse_order, &edge_collapses[0..edge_collapse_count]);

    let triangle_collapse_goal = (result_count - target_index_count) / 3;

    for (i, r) in collapse_remap.iter_mut().enumerate() {
      *r = i as u32;
    }

    collapse_locked.fill(false);

    let collapses_count = perform_edge_collapses(
      &mut collapse_remap,
      &mut collapse_locked,
      &mut vertex_quadrics,
      &edge_collapses[0..edge_collapse_count],
      &collapse_order[0..edge_collapse_count],
      &remap,
      &wedge,
      &vertex_kind,
      &vertex_positions,
      &adjacency,
      triangle_collapse_goal,
      error_limit,
      &border_loop,
      &mut result_error,
    );

    // no edges can be collapsed any more due to hitting the error limit or triangle collapse limit
    if collapses_count == 0 {
      break;
    }

    border_loop.remap_edge_loops(&collapse_remap);

    let new_count = remap_index_buffer(&mut result[0..result_count], &collapse_remap);
    assert!(new_count < result_count);

    result_count = new_count;
  }

  // result_error is quadratic; we need to remap it back to linear
  let out_result_error = result_error.sqrt() * error_scale;

  EdgeCollapseResult {
    result_error: out_result_error,
    result_count,
  }
}

/// rescale the vertex into unit cube with min(0,0,0)
fn rescale_positions<Vertex>(vertices: &[Vertex]) -> (f32, Vec<Vec3<f32>>)
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let bbox: Box3 = vertices.iter().map(|v| v.position()).collect();
  let box_size = bbox.size();
  let extent = box_size.x.max(box_size.y).max(box_size.z);
  let scale = inverse_or_zeroed(extent);

  let positions = vertices
    .iter()
    .map(|v| (v.position() - bbox.min) * scale)
    .collect();

  (extent, positions)
}

#[derive(Clone, Default)]
struct Collapse {
  v0: u32,
  v1: u32,
  error: f32,
}

fn iter_triangle_edges(tri: [u32; 3]) -> impl Iterator<Item = (u32, u32)> {
  [1, 2, 0]
    .into_iter()
    .enumerate()
    .map(move |(i, e)| (tri[i], tri[e]))
}

fn pick_edge_collapses(
  collapses: &mut [Collapse],
  indices: &[u32],
  vertex_positions: &[Vec3<f32>],
  vertex_quadrics: &[Quadric],
  remap: &[u32],
  vertex_kind: &[VertexKind],
  borders: &BorderLoops,
) -> usize {
  let mut collapse_count = 0;

  for triangle in indices.array_chunks::<3>() {
    for (i0, i1) in iter_triangle_edges(*triangle) {
      let i0 = i0 as usize;
      let i1 = i1 as usize;

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
      if !(k0.can_collapse_into(k1) || k1.can_collapse_into(k0)) {
        continue;
      }

      // manifold and seam edges should occur twice (i0->i1 and i1->i0) - skip redundant edges
      if VertexKind::has_opposite_edge(k0, k1) && remap[i1] > remap[i0] {
        continue;
      }

      // two vertices are on a border or a seam, but there's no direct edge between them
      // this indicates that they belong to two different edge loops and we should not collapse this
      // edge loop[] tracks half edges so we only need to check i0->i1
      if k0 == k1
        && (k0 == VertexKind::Border || k0 == VertexKind::SimpleSeam)
        && borders.get_half_edge_out_vertex(i0 as u32) != i1 as u32
      {
        continue;
      }

      if k0 == VertexKind::Locked || k1 == VertexKind::Locked {
        // the same check as above, but for border/seam -> locked collapses
        // loop[] and loopback[] track half edges so we only need to check one of them
        if (k0 == VertexKind::Border || k0 == VertexKind::SimpleSeam)
          && borders.get_half_edge_out_vertex(i0 as u32) != i1 as u32
        {
          continue;
        }

        if (k1 == VertexKind::Border || k1 == VertexKind::SimpleSeam)
          && borders.get_half_edge_in_vertex(i1 as u32) != i0 as u32
        {
          continue;
        }
      }

      // edge can be collapsed in either direction - we will pick the one with minimum error
      // note: we evaluate error later during collapse ranking, here we just tag the edge as
      // bidirectional
      let c = if k0.can_collapse_into(k1) && k1.can_collapse_into(k0) {
        create_collapse_request(
          (i0 as u32, i1 as u32),
          true,
          vertex_positions,
          vertex_quadrics,
          remap,
        )
      } else {
        // edge can only be collapsed in one direction
        let e0 = if k0.can_collapse_into(k1) { i0 } else { i1 };
        let e1 = if k0.can_collapse_into(k1) { i1 } else { i0 };

        create_collapse_request(
          (e0 as u32, e1 as u32),
          false,
          vertex_positions,
          vertex_quadrics,
          remap,
        )
      };

      collapses[collapse_count] = c;
      collapse_count += 1;
    }
  }

  collapse_count
}

fn create_collapse_request(
  collapsable_pair: (u32, u32),
  collapse_is_bidirectional: bool,
  vertex_positions: &[Vec3<f32>],
  vertex_quadrics: &[Quadric],
  remap: &[u32],
) -> Collapse {
  let (i0, i1) = collapsable_pair;

  // most edges are bidirectional which means we need to evaluate errors for two collapses
  // to keep this code branchless we just use the same edge for unidirectional edges
  let j0 = if collapse_is_bidirectional { i1 } else { i0 };
  let j1 = if collapse_is_bidirectional { i0 } else { i1 };

  let qi = vertex_quadrics[remap[i0 as usize] as usize];
  let qj = vertex_quadrics[remap[j0 as usize] as usize];

  let ei = qi.error(&vertex_positions[i1 as usize]);
  let ej = if collapse_is_bidirectional {
    qj.error(&vertex_positions[j1 as usize])
  } else {
    f32::MAX
  };

  Collapse {
    // pick edge direction with minimal error
    v0: if ei <= ej { i0 } else { j0 },
    v1: if ei <= ej { i1 } else { j1 },
    error: ei.min(ej),
  }
}

#[allow(clippy::needless_range_loop)]
fn sort_edge_collapses(sort_order: &mut [u32], collapses: &[Collapse]) {
  const SORT_BITS: usize = 12;
  const SORT_BINS: usize = 2048 + 512; // exponent range [-127, 32)

  // fill histogram for counting sort
  let mut histogram = [0u32; SORT_BINS];

  for c in collapses {
    // skip sign bit since error is non-negative
    let error = c.error.to_bits();
    let key = (error << 1) >> (32 - SORT_BITS);
    let key = if key < SORT_BINS as u32 {
      key
    } else {
      SORT_BINS as u32 - 1
    } as usize;

    histogram[key] += 1;
  }

  // compute offsets based on histogram data
  let mut histogram_sum = 0;

  for i in 0..SORT_BINS {
    let count = histogram[i];
    histogram[i] = histogram_sum;
    histogram_sum += count;
  }

  assert_eq!(histogram_sum as usize, collapses.len());

  // compute sort order based on offsets
  for (i, c) in collapses.iter().enumerate() {
    // skip sign bit since error is non-negative
    let error = c.error.to_bits();
    let key = (error << 1) >> (32 - SORT_BITS);
    let key = if key < SORT_BINS as u32 {
      key
    } else {
      SORT_BINS as u32 - 1
    } as usize;

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
  wedge: &VertexWedgeLoops,
  vertex_kind: &[VertexKind],
  vertex_positions: &[Vec3<f32>],
  adjacency: &EdgeAdjacency,
  triangle_collapse_goal: usize,
  error_limit: f32,
  borders: &BorderLoops,
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

    let error = c.error;

    if error > error_limit {
      break;
    }

    if triangle_collapses >= triangle_collapse_goal {
      break;
    }

    // we limit the error in each pass based on the error of optimal last collapse; since many
    // collapses will be locked as they will share vertices with other successful collapses, we
    // need to increase the acceptable error by some factor
    let error_goal = if edge_collapse_goal < collapse_count {
      let c_ = &collapses[collapse_order[edge_collapse_goal] as usize];
      1.5 * c_.error
    } else {
      f32::MAX
    };

    // on average, each collapse is expected to lock 6 other collapses; to avoid degenerate passes
    // on meshes with odd topology, we only abort if we got over 1/6 collapses accordingly.
    if error > error_goal
      && error > *result_error
      && triangle_collapses > triangle_collapse_goal / 6
    {
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
        // remap all vertices in the complex to the target vertex
        let mut v = i0;

        loop {
          collapse_remap[v] = i1 as u32;
          v = wedge.next_same_position_vertex(v) as usize;

          if v == i0 {
            break;
          }
        }
      }
      VertexKind::SimpleSeam => {
        // for seam collapses we need to move the seam pair together; this is a bit tricky since
        // we need to rely on edge loops as target vertex may be locked (and thus have more than two wedges)
        let s0 = wedge.next_same_position_vertex(i0) as usize;
        let s1 = if borders.get_half_edge_out_vertex(i0 as u32) == i1 as u32 {
          borders.get_half_edge_in_vertex(s0 as u32)
        } else {
          borders.get_half_edge_out_vertex(s0 as u32)
        };

        assert!(s0 != i0);
        assert!(wedge.next_same_position_vertex(s0) as usize == i0);
        assert!(s1 != u32::MAX);
        assert!(remap[s1 as usize] == r1 as u32);

        collapse_remap[i0] = i1 as u32;
        collapse_remap[s0] = s1;
      }
      _ => {
        assert_eq!(wedge.next_same_position_vertex(i0) as usize, i0);

        collapse_remap[i0] = i1 as u32;
      }
    }

    // note: we technically don't need to lock r1 if it's a locked vertex, as it can't move and its quadric won't be used
    // however, this results in slightly worse error on some meshes because the locked collapses
    // get an unfair advantage wrt scheduling
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

  let ndp = nbc.dot(nbd);
  let abc = nbc.dot(nbc);
  let abd = nbd.dot(nbd);

  // scale is cos(angle); somewhat arbitrarily set to ~75 degrees
  // note that the "pure" check is ndp <= 0 (90 degree cutoff) but that allows flipping through a series of close-to-90 collapses
  ndp <= 0.25 * (abc * abd).sqrt()
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
