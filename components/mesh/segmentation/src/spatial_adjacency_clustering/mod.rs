mod connectivity;
use connectivity::*;

use crate::*;

// This must be <= 255 since index 0xff is used internally to index a vertex that doesn't belong to
// a meshlet
pub const MESHLET_MAX_VERTICES: u8 = 255;

// A reasonable limit is around 2*max_vertices or less
pub const MESHLET_MAX_TRIANGLES: u32 = 512;

#[derive(Default, Clone, Copy)]
pub struct Meshlet {
  // offsets within meshlet_vertices and meshlet_triangles arrays with meshlet data
  pub vertex_offset: u32,
  pub triangle_offset: u32,

  // number of vertices and triangles used in the meshlet; data is stored in consecutive range
  // defined by offset and count
  pub vertex_count: u32,
  pub triangle_count: u32,
}

pub struct Config {
  pub max_vertices: u8,
  /// should <= 512
  pub max_triangles: u32,
  /// cone_weight should be set to 0 when cone culling is not used, and a value between 0 and 1
  /// otherwise to balance between cluster size and cone culling efficiency
  pub cone_weight: f32,
}

pub fn build_meshlets<V: Positioned<Position = Vec3<f32>>, SA: SpaceSearchAcceleration>(
  config: Config,
  indices: &[u32],
  vertices: &[V],
  meshlets: &mut [Meshlet],
  meshlet_vertices: &mut [u32],
  meshlet_triangles: &mut [u8],
) -> usize {
  assert!(indices.len() / 3 == 0);
  assert!(indices.len() >= 3);

  assert!(config.max_vertices >= 3);
  assert!(config.max_triangles >= 1 && config.max_triangles <= MESHLET_MAX_TRIANGLES);
  // ensures the caller will compute output space properly as index data is 4b aligned
  assert!(config.max_triangles % 4 == 0);
  assert!(config.cone_weight >= 0. && config.cone_weight <= 1.);

  let mut adjacency = TriangleAdjacency::new(indices, vertices.len());

  let mut live_triangles = adjacency.counts.clone();

  let face_count = indices.len() / 3;
  let mut emitted_flags = vec![false; face_count];

  // for each triangle, precompute centroid & normal to use for scoring
  let (triangles, mesh_area) = compute_triangle_cones(indices, vertices);
  let triangle_area_avg = mesh_area / face_count as f32 * 0.5;
  // assuming each meshlet is a square patch, expected radius is sqrt(expected area)
  let meshlet_expected_radius = (triangle_area_avg * config.max_triangles as f32).sqrt() * 0.5;

  // todo accel
  let space_search = SA::build(indices, vertices);

  // index of the vertex in the meshlet, 0xff if the vertex isn't used
  let mut used: Vec<u8> = vec![0xff; vertices.len()];
  let mut meshlet_offset = 0;
  let mut meshlet_cone_acc = Cone::default();
  let mut meshlet = Meshlet::default();

  loop {
    let meshlet_cone = get_meshlet_cone(&meshlet_cone_acc, meshlet.triangle_count);

    let mut best_extra = 0;
    let mut best_triangle = get_neighbor_triangle(
      &meshlet,
      Some(&meshlet_cone),
      meshlet_vertices,
      indices,
      &adjacency,
      &triangles,
      &live_triangles,
      &used,
      meshlet_expected_radius,
      config.cone_weight,
      Some(&mut best_extra),
    );

    // if the best triangle doesn't fit into current meshlet, the spatial scoring we've used is
    // not very meaningful, so we re-select using topological scoring
    if best_triangle != !0
      && (meshlet.vertex_count + best_extra > config.max_vertices as u32
        || meshlet.triangle_count >= config.max_triangles)
    {
      best_triangle = get_neighbor_triangle(
        &meshlet,
        None,
        meshlet_vertices,
        indices,
        &adjacency,
        &triangles,
        &live_triangles,
        &used,
        meshlet_expected_radius,
        0.,
        None,
      );
    }

    // when we run out of neighboring triangles we need to switch to spatial search; we currently
    // just pick the closest triangle irrespective of connectivity
    if best_triangle == !0 {
      best_triangle =
        space_search.search_nearest(meshlet_cone.position, |index| emitted_flags[index as usize]);
    }

    if best_triangle == !0 {
      break;
    }

    let best_triangle = best_triangle as usize;

    let a = indices[best_triangle * 3] as usize;
    let b = indices[best_triangle * 3 + 1] as usize;
    let c = indices[best_triangle * 3 + 2] as usize;
    assert!(a < vertices.len() && b < vertices.len() && c < vertices.len());

    // add meshlet to the output; when the current meshlet is full we reset the accumulated bounds
    if append_meshlet(
      &mut meshlet,
      a as u32,
      b as u32,
      c as u32,
      &mut used,
      meshlets,
      meshlet_vertices,
      meshlet_triangles,
      meshlet_offset,
      config.max_vertices as u32,
      config.max_triangles,
    ) {
      meshlet_offset += 1;
      meshlet_cone_acc = Default::default();
    }

    live_triangles[a] -= 1;
    live_triangles[b] -= 1;
    live_triangles[c] -= 1;

    // remove emitted triangle from adjacency data
    // this makes sure that we spend less time traversing these lists on subsequent iterations
    for k in 0..3 {
      let index = indices[best_triangle * 3 + k] as usize;

      let start = adjacency.offsets[index] as usize;
      let count = adjacency.counts[index] as usize;
      let neighbors = adjacency.face_ids.get_mut(start..start + count).unwrap();
      let last = neighbors[count - 1];

      for tri in neighbors {
        if *tri as usize == best_triangle {
          *tri = last;
          adjacency.counts[index] -= 1;
          break;
        }
      }
    }

    // update aggregated meshlet cone data for scoring subsequent triangles
    meshlet_cone_acc.position += triangles[best_triangle].position;
    meshlet_cone_acc.direction += triangles[best_triangle].direction;

    emitted_flags[best_triangle] = true;
  }

  if meshlet.triangle_count > 0 {
    finish_meshlet(&meshlet, meshlet_triangles);

    meshlets[meshlet_offset as usize] = meshlet;
    meshlet_offset += 1;
  }

  // assert!(meshlet_offset <= meshopt_buildMeshletsBound(index_count, max_vertices,
  // max_triangles));
  meshlet_offset as usize
}

pub trait SpaceSearchAcceleration {
  fn build<V>(indices: &[u32], vertices: &[V]) -> Self;
  fn search_nearest(&self, position: Vec3<f32>, should_skip: impl Fn(u32) -> bool) -> u32;
}

fn finish_meshlet(meshlet: &Meshlet, meshlet_triangles: &mut [u8]) {
  let mut offset = meshlet.triangle_offset + meshlet.triangle_count * 3;

  // fill 4b padding with 0
  while offset & 3 == 0 {
    meshlet_triangles[offset as usize] = 0;
    offset += 1;
  }
}

fn append_meshlet(
  meshlet: &mut Meshlet,
  a: u32,
  b: u32,
  c: u32,
  used: &mut [u8],
  meshlets: &mut [Meshlet],
  meshlet_vertices: &mut [u32],
  meshlet_triangles: &mut [u8],
  meshlet_offset: u32,
  max_vertices: u32,
  max_triangles: u32,
) -> bool {
  let av = used[a as usize];
  let bv = used[b as usize];
  let cv = used[c as usize];

  let mut result = false;

  let used_extra = (av == 0xff) as u32 + (bv == 0xff) as u32 + (cv == 0xff) as u32;

  if meshlet.vertex_count + used_extra > max_vertices || meshlet.triangle_count >= max_triangles {
    meshlets[meshlet_offset as usize] = *meshlet;

    for j in 0..meshlet.vertex_count {
      used[meshlet_vertices[(meshlet.vertex_offset + j) as usize] as usize] = 0xff;
    }

    finish_meshlet(meshlet, meshlet_triangles);

    meshlet.vertex_offset += meshlet.vertex_count;
    meshlet.triangle_offset += (meshlet.triangle_count * 3 + 3) & !3; // 4b padding
    meshlet.vertex_count = 0;
    meshlet.triangle_count = 0;

    result = true;
  }

  if av == 0xff {
    used[a as usize] = meshlet.vertex_count as u8;
    meshlet_vertices[(meshlet.vertex_offset + meshlet.vertex_count) as usize] = a;
    meshlet.vertex_count += 1;
  }

  if bv == 0xff {
    used[b as usize] = meshlet.vertex_count as u8;
    meshlet_vertices[(meshlet.vertex_offset + meshlet.vertex_count) as usize] = b;
    meshlet.vertex_count += 1;
  }

  if cv == 0xff {
    used[c as usize] = meshlet.vertex_count as u8;
    meshlet_vertices[(meshlet.vertex_offset + meshlet.vertex_count) as usize] = c;
    meshlet.vertex_count += 1;
  }

  meshlet_triangles[(meshlet.triangle_offset + meshlet.triangle_count * 3) as usize] = av;
  meshlet_triangles[(meshlet.triangle_offset + meshlet.triangle_count * 3 + 1) as usize] = bv;
  meshlet_triangles[(meshlet.triangle_offset + meshlet.triangle_count * 3 + 2) as usize] = cv;
  meshlet.triangle_count += 1;

  result
}

fn get_neighbor_triangle(
  meshlet: &Meshlet,
  meshlet_cone: Option<&Cone>,
  meshlet_vertices: &[u32],
  indices: &[u32],
  adjacency: &TriangleAdjacency,
  triangles: &[Cone],
  live_triangles: &[u32],
  used: &[u8],
  meshlet_expected_radius: f32,
  cone_weight: f32,
  out_extra: Option<&mut u32>,
) -> u32 {
  let mut best_triangle = !0;
  let mut best_extra = 5;
  let mut best_score = f32::MAX;

  for i in 0..meshlet.vertex_count {
    let index = meshlet_vertices[(meshlet.vertex_offset + i) as usize];

    for triangle in adjacency.iter_adjacency_faces(index as usize) {
      let triangle = triangle as usize;
      let a = indices[triangle * 3] as usize;
      let b = indices[triangle * 3 + 1] as usize;
      let c = indices[triangle * 3 + 2] as usize;

      let mut extra =
        (used[a] == 0xff) as u32 + (used[b] == 0xff) as u32 + (used[c] == 0xff) as u32;

      // triangles that don't add new vertices to meshlets are max. priority
      if extra != 0 {
        // artificially increase the priority of dangling triangles as they're expensive to add to
        // new meshlets
        if live_triangles[a] == 1 || live_triangles[b] == 1 || live_triangles[c] == 1 {
          extra = 0;
        }

        extra += 1;
      }

      // since topology-based priority is always more important than the score, we can skip scoring
      // in some cases
      if extra > best_extra {
        continue;
      }

      // caller selects one of two scoring functions: geometrical (based on meshlet cone) or
      // topological (based on remaining triangles)
      let score = if let Some(meshlet_cone) = meshlet_cone {
        let tri_cone = &triangles[triangle];

        let distance2 = (tri_cone.position - meshlet_cone.position).length2();
        let spread = tri_cone.direction.dot(meshlet_cone.direction);

        get_meshlet_score(distance2, spread, cone_weight, meshlet_expected_radius)
      } else {
        // each live_triangles entry is >= 1 since it includes the current triangle we're processing
        (live_triangles[a] + live_triangles[b] + live_triangles[c] - 3) as f32
      };

      // note that topology-based priority is always more important than the score
      // this helps maintain reasonable effectiveness of meshlet data and reduces scoring cost
      if extra < best_extra || score < best_score {
        best_triangle = triangle;
        best_extra = extra;
        best_score = score;
      }
    }
  }

  if let Some(out_extra) = out_extra {
    *out_extra = best_extra;
  }

  best_triangle as u32
}

fn get_meshlet_score(distance2: f32, spread: f32, cone_weight: f32, expected_radius: f32) -> f32 {
  let cone = 1.0 - spread * cone_weight;
  let cone_clamped = if cone < 1e-3 { 1e-3 } else { cone };

  (1. + distance2.sqrt() / expected_radius * (1. - cone_weight)) * cone_clamped
}
