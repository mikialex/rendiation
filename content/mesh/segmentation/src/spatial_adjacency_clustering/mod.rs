mod connectivity;
use connectivity::*;

mod bounding;
use bounding::*;
mod space_search;
pub use space_search::*;
mod seed;
use seed::*;

use crate::*;

// A reasonable limit is around 2*max_vertices or less
pub const MESHLET_MAX_TRIANGLES: u32 = 512;

#[derive(Default, Clone, Copy)]
pub struct Meshlet {
  // offsets within meshlet_vertices and meshlet_triangles arrays with meshlet data
  pub vertex_offset: u32,
  // todo naming issue, this actually index offset
  pub triangle_offset: u32,

  // number of vertices and triangles used in the meshlet; data is stored in consecutive range
  // defined by offset and count
  pub vertex_count: u32,
  pub triangle_count: u32,
}

pub struct ClusteringConfig {
  pub max_vertices: u8,
  /// should <= [MESHLET_MAX_TRIANGLES]
  pub max_triangles: u32,
  /// cone_weight should be set to 0 when cone culling is not used, and a value between 0 and 1
  /// otherwise to balance between cluster size and cone culling efficiency
  pub cone_weight: f32,
}

impl ClusteringConfig {
  pub fn validate(&self) -> bool {
    self.max_triangles >= 3
      && self.max_vertices >= 1
      && self.max_triangles <= MESHLET_MAX_TRIANGLES
      // ensures the caller will compute output space properly as index data is 4b aligned
      && self.max_triangles % 4 == 0
      && self.cone_weight >= 0.
      && self.cone_weight <= 1.
  }
}

pub fn build_meshlets_bound(index_count: usize, config: &ClusteringConfig) -> usize {
  assert!(index_count % 3 == 0);
  assert!(index_count > 0);
  assert!(config.validate());

  // meshlet construction is limited by max vertices and max triangles per meshlet
  // the worst case is that the input is an unindexed stream since this equally stresses both limits
  // note that we assume that in the worst case, we leave 2 vertices unpacked in each meshlet - if
  // we have space for 3 we can pack any triangle
  let max_vertices_conservative = (config.max_vertices - 2) as usize;
  let meshlet_limit_vertices = index_count.div_ceil(max_vertices_conservative);
  let meshlet_limit_triangles = (index_count / 3).div_ceil(config.max_triangles as usize);

  if meshlet_limit_vertices > meshlet_limit_triangles {
    meshlet_limit_vertices
  } else {
    meshlet_limit_triangles
  }
}

/// return the built meshlet count written into meshlets array
pub fn build_meshlets<V: Positioned<Position = Vec3<f32>>, SA: SpaceSearchAcceleration<V>>(
  config: &ClusteringConfig,
  indices: &[u32],
  vertices: &[V],
  meshlets: &mut [Meshlet],
  meshlet_vertices: &mut [u32],
  meshlet_triangles: &mut [u8],
) -> usize {
  config.validate();
  assert!(indices.len() % 3 == 0);
  assert!(indices.len() >= 3);

  if indices.is_empty() {
    return 0;
  }

  let mut adjacency = TriangleAdjacency::new(indices, vertices.len());

  let face_count = indices.len() / 3;
  let mut emitted_flags = vec![false; face_count];

  // for each triangle, precompute centroid & normal to use for scoring
  let (triangles, mesh_area) = compute_triangle_cones(indices, vertices);
  let triangle_area_avg = mesh_area / face_count as f32 * 0.5;
  // assuming each meshlet is a square patch, expected radius is sqrt(expected area)
  let meshlet_expected_radius = (triangle_area_avg * config.max_triangles as f32).sqrt() * 0.5;

  let space_search = SA::build(indices, vertices);
  let (mut seeding, initial_seed) = MeshletBuildSeeding::new(triangles.iter().map(|t| t.position));

  // index of the vertex in the meshlet, 0xff if the vertex isn't used
  let mut used: Vec<u8> = vec![0xff; vertices.len()];
  let mut meshlet_offset = 0;
  let mut meshlet_cone_acc = Cone::default();
  let mut meshlet = Meshlet::default();

  loop {
    let meshlet_cone = get_meshlet_cone(&meshlet_cone_acc, meshlet.triangle_count);

    // for the first triangle, we don't have a meshlet cone yet, so we use the initial seed
    // to continue the meshlet, we select an adjacent triangle based on connectivity and spatial scoring
    let mut best_triangle = if meshlet_offset == 0 && meshlet.triangle_count == 0 {
      initial_seed
    } else {
      get_neighbor_triangle(
        &meshlet,
        &meshlet_cone,
        meshlet_vertices,
        indices,
        &adjacency,
        &triangles,
        &used,
        meshlet_expected_radius,
        config.cone_weight,
      )
    };

    // when we run out of adjacent triangles we need to switch to spatial search; we currently
    // just pick the closest triangle irrespective of connectivity
    if best_triangle == !0 {
      best_triangle = space_search.search_nearest(
        meshlet_cone.position,
        |index| emitted_flags[index as usize],
        indices,
        vertices,
      );
    }

    if best_triangle == !0 {
      break;
    } else {
      assert!(!emitted_flags[best_triangle as usize]);
    }

    let best_triangle_ = best_triangle as usize;
    let a = indices[best_triangle_ * 3] as usize;
    let b = indices[best_triangle_ * 3 + 1] as usize;
    let c = indices[best_triangle_ * 3 + 2] as usize;

    let best_extra = (used[a] == 0xff) as u32 + (used[b] == 0xff) as u32 + (used[c] == 0xff) as u32;

    // if the best triangle doesn't fit into current meshlet, we re-select using seeds to maintain global flow
    if meshlet.vertex_count + best_extra > config.max_vertices as u32
      || meshlet.triangle_count >= config.max_triangles
    {
      seeding.prune(&emitted_flags);

      let meshlet_vertices_range =
        meshlet.vertex_offset as usize..(meshlet.vertex_offset + meshlet.vertex_count) as usize;
      let meshlet_vertices_range = meshlet_vertices.get(meshlet_vertices_range).unwrap();

      let triangle_position = |tri_id| triangles[tri_id as usize].position;

      seeding.append(
        meshlet_vertices_range,
        indices,
        &adjacency,
        triangle_position,
      );

      let best_seed = seeding.select_best(
        indices,
        adjacency.vertex_referenced_face_counts(),
        triangle_position,
      );

      // we may not find a valid seed triangle if the mesh is disconnected as seeds are based on adjacency
      if best_seed != u32::MAX {
        best_triangle = best_seed
      }
    }

    let best_triangle_ = best_triangle as usize;
    let a = indices[best_triangle_ * 3] as usize;
    let b = indices[best_triangle_ * 3 + 1] as usize;
    let c = indices[best_triangle_ * 3 + 2] as usize;

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
      config,
    ) {
      meshlet_offset += 1;
      meshlet_cone_acc = Default::default();
    }

    let best_triangle = best_triangle as usize;

    // this makes sure that we spend less time traversing these lists on subsequent iterations
    adjacency.update_by_remove_a_triangle(best_triangle, indices);

    // update aggregated meshlet cone data for scoring subsequent triangles
    meshlet_cone_acc.position += triangles[best_triangle].position;
    meshlet_cone_acc.direction += triangles[best_triangle].direction;

    emitted_flags[best_triangle] = true;
  }

  if meshlet.triangle_count > 0 {
    meshlets[meshlet_offset as usize] = meshlet;
    meshlet_offset += 1;
  }

  assert!(meshlet_offset as usize <= build_meshlets_bound(indices.len(), config));
  meshlet_offset as usize
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
  config: &ClusteringConfig,
) -> bool {
  let mut result = false;

  let a = a as usize;
  let b = b as usize;
  let c = c as usize;

  let used_extra = (used[a] == 0xff) as u32 + (used[b] == 0xff) as u32 + (used[c] == 0xff) as u32;

  if meshlet.vertex_count + used_extra > config.max_vertices as u32
    || meshlet.triangle_count >= config.max_triangles
  {
    meshlets[meshlet_offset as usize] = *meshlet;

    for j in 0..meshlet.vertex_count {
      used[meshlet_vertices[(meshlet.vertex_offset + j) as usize] as usize] = 0xff;
    }

    meshlet.vertex_offset += meshlet.vertex_count;
    meshlet.triangle_offset += meshlet.triangle_count * 3;
    meshlet.vertex_count = 0;
    meshlet.triangle_count = 0;

    result = true;
  }

  if used[a] == 0xff {
    used[a] = meshlet.vertex_count as u8;
    meshlet_vertices[(meshlet.vertex_offset + meshlet.vertex_count) as usize] = a as u32;
    meshlet.vertex_count += 1;
  }

  if used[b] == 0xff {
    used[b] = meshlet.vertex_count as u8;
    meshlet_vertices[(meshlet.vertex_offset + meshlet.vertex_count) as usize] = b as u32;
    meshlet.vertex_count += 1;
  }

  if used[c] == 0xff {
    used[c] = meshlet.vertex_count as u8;
    meshlet_vertices[(meshlet.vertex_offset + meshlet.vertex_count) as usize] = c as u32;
    meshlet.vertex_count += 1;
  }

  let av = used[a];
  let bv = used[b];
  let cv = used[c];

  meshlet_triangles[(meshlet.triangle_offset + meshlet.triangle_count * 3) as usize] = av;
  meshlet_triangles[(meshlet.triangle_offset + meshlet.triangle_count * 3 + 1) as usize] = bv;
  meshlet_triangles[(meshlet.triangle_offset + meshlet.triangle_count * 3 + 2) as usize] = cv;
  meshlet.triangle_count += 1;

  result
}

fn get_neighbor_triangle(
  meshlet: &Meshlet,
  meshlet_cone: &Cone,
  meshlet_vertices: &[u32],
  indices: &[u32],
  adjacency: &TriangleAdjacency,
  triangles: &[Cone],
  used: &[u8],
  meshlet_expected_radius: f32,
  cone_weight: f32,
) -> u32 {
  let live_triangles = adjacency.vertex_referenced_face_counts();

  let mut best_triangle = !0;
  let mut best_priority = 5;
  let mut best_score = f32::MAX;

  for i in 0..meshlet.vertex_count {
    let index = meshlet_vertices[(meshlet.vertex_offset + i) as usize];

    for triangle in adjacency.iter_adjacency_faces(index) {
      let triangle = triangle as usize;
      let a = indices[triangle * 3] as usize;
      let b = indices[triangle * 3 + 1] as usize;
      let c = indices[triangle * 3 + 2] as usize;

      let extra = (used[a] == 0xff) as u32 + (used[b] == 0xff) as u32 + (used[c] == 0xff) as u32;
      assert!(extra <= 2);

      // triangles that don't add new vertices to meshlets are max priority
      let priority = if extra == 0 {
        0
      // artificially increase the priority of dangling triangles as they're expensive to add to
      } else if live_triangles[a] == 1 || live_triangles[b] == 1 || live_triangles[c] == 1 {
        1
      // if two vertices have live count of 2, removing this triangle will make another triangle
      // dangling which is good for overall flow
      } else if (live_triangles[a] == 2) as u32
        + (live_triangles[b] == 2) as u32
        + (live_triangles[c] == 2) as u32
        >= 2
      {
        1 + extra
      // otherwise adjust priority to be after the above cases, 3 or 4 based on used[] count
      } else {
        2 + extra
      };

      // since topology-based priority is always more important than the score, we can skip scoring
      // in some cases
      if priority > best_priority {
        continue;
      }

      let tri_cone = &triangles[triangle];
      let dx = tri_cone.position - meshlet_cone.position;
      let distance = if cone_weight < 0. {
        dx.map(|v| v.abs()).max_channel()
      } else {
        dx.length()
      };
      let spread = tri_cone.direction.dot(meshlet_cone.direction);
      let score = get_meshlet_score(distance, spread, cone_weight, meshlet_expected_radius);

      // note that topology-based priority is always more important than the score
      // this helps maintain reasonable effectiveness of meshlet data and reduces scoring cost
      if priority < best_priority || score < best_score {
        best_triangle = triangle;
        best_priority = priority;
        best_score = score;
      }
    }
  }

  best_triangle as u32
}

fn get_meshlet_score(distance: f32, spread: f32, cone_weight: f32, expected_radius: f32) -> f32 {
  if cone_weight < 0. {
    return 1. + distance / expected_radius;
  }

  let cone = 1.0 - spread * cone_weight;
  let cone_clamped = cone.max(1e-3);

  (1. + distance / expected_radius * (1. - cone_weight)) * cone_clamped
}
