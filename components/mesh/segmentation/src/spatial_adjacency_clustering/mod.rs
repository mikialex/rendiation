mod connectivity;
use connectivity::*;

use crate::*;

// This must be <= 255 since index 0xff is used internally to index a vertex that doesn't belong to
// a meshlet
const MESHLET_MAX_VERTICES: u8 = 255;

// A reasonable limit is around 2*max_vertices or less
const MESHLET_MAX_TRIANGLES: u32 = 512;

#[derive(Default)]
pub struct Meshlet {
  // offsets within meshlet_vertices and meshlet_triangles arrays with meshlet data
  vertex_offset: u32,
  triangle_offset: u32,

  // number of vertices and triangles used in the meshlet; data is stored in consecutive range
  // defined by offset and count
  vertex_count: u32,
  triangle_count: u32,
}

pub struct Config {
  max_vertices: u8,
  /// should <= 512
  max_triangles: u32,
  /// cone_weight should be set to 0 when cone culling is not used, and a value between 0 and 1
  /// otherwise to balance between cluster size and cone culling efficiency
  cone_weight: f32,
}

pub fn build_meshlets<V: Positioned<Position = Vec3<f32>>, SA: SpaceSearchAcceleration>(
  config: Config,
  indices: &[u32],
  vertices: &[V],
  meshlets: &mut [Meshlet],
  meshlet_vertices: &mut [V],
  meshlet_triangles: &mut [u8],
) {
  assert!(indices.len() / 3 == 0);
  assert!(indices.len() >= 3);

  assert!(config.max_vertices >= 3);
  assert!(config.max_triangles >= 1 && config.max_triangles <= MESHLET_MAX_TRIANGLES);
  // ensures the caller will compute output space properly as index data is 4b aligned
  assert!(config.max_triangles % 4 == 0);
  assert!(config.cone_weight >= 0. && config.cone_weight <= 1.);

  let adjacency = TriangleAdjacency::new(indices, vertices.len());

  let live_triangles = adjacency.counts.clone();

  let face_count = indices.len() / 3;
  let emit_flags = vec![false; face_count];

  // for each triangle, precompute centroid & normal to use for scoring
  let (triangles, mesh_area) = compute_triangle_cones(indices, vertices);
  let triangle_area_avg = mesh_area / face_count as f32 * 0.5;
  // assuming each meshlet is a square patch, expected radius is sqrt(expected area)
  let meshlet_expected_radius = (triangle_area_avg * config.max_triangles as f32).sqrt() * 0.5;

  // todo accel
  let space_search = SA::build(indices, vertices);

  // index of the vertex in the meshlet, 0xff if the vertex isn't used
  let mut used = vec![0xff; vertices.len()];
  let mut meshlet_offset = 0;
  let mut meshlet_cone_acc = Cone::default();
  let mut meshlet = Meshlet::default();

  loop {
    let meshlet_cone = get_meshlet_cone(&meshlet_cone_acc, meshlet.triangle_count);

    let mut best_extra = 0;
    // 		unsigned int best_triangle = getNeighborTriangle(meshlet, &meshlet_cone, meshlet_vertices,
    // indices, adjacency, triangles, live_triangles, used, meshlet_expected_radius, cone_weight,
    // &best_extra);

    // 		// if the best triangle doesn't fit into current meshlet, the spatial scoring we've used is
    // not very meaningful, so we re-select using topological scoring 		if (best_triangle != ~0u
    // && (meshlet.vertex_count + best_extra > max_vertices || meshlet.triangle_count >=
    // max_triangles)) 		{ 			best_triangle = getNeighborTriangle(meshlet, NULL, meshlet_vertices,
    // indices, adjacency, triangles, live_triangles, used, meshlet_expected_radius, 0.f, NULL);
    // }

    // 		// when we run out of neighboring triangles we need to switch to spatial search; we currently
    // just pick the closest triangle irrespective of connectivity 		if (best_triangle == ~0u)
    // 		{
    // 			float position[3] = {meshlet_cone.px, meshlet_cone.py, meshlet_cone.pz};
    // 			unsigned int index = ~0u;
    // 			float limit = FLT_MAX;

    // 			kdtreeNearest(nodes, 0, &triangles[0].px, sizeof(Cone) / sizeof(float), emitted_flags,
    // position, index, limit);

    // 			best_triangle = index;
    // 		}

    // 		if (best_triangle == ~0u)
    // 			break;

    // 		unsigned int a = indices[best_triangle * 3 + 0], b = indices[best_triangle * 3 + 1], c =
    // indices[best_triangle * 3 + 2]; 		assert(a < vertex_count && b < vertex_count && c <
    // vertex_count);

    // 		// add meshlet to the output; when the current meshlet is full we reset the accumulated
    // bounds 		if (appendMeshlet(meshlet, a, b, c, used, meshlets, meshlet_vertices,
    // meshlet_triangles, meshlet_offset, max_vertices, max_triangles)) 		{
    // 			meshlet_offset++;
    // 			memset(&meshlet_cone_acc, 0, sizeof(meshlet_cone_acc));
    // 		}

    // 		live_triangles[a]--;
    // 		live_triangles[b]--;
    // 		live_triangles[c]--;

    // 		// remove emitted triangle from adjacency data
    // 		// this makes sure that we spend less time traversing these lists on subsequent iterations
    // 		for (size_t k = 0; k < 3; ++k)
    // 		{
    // 			unsigned int index = indices[best_triangle * 3 + k];

    // 			unsigned int* neighbors = &adjacency.data[0] + adjacency.offsets[index];
    // 			size_t neighbors_size = adjacency.counts[index];

    // 			for (size_t i = 0; i < neighbors_size; ++i)
    // 			{
    // 				unsigned int tri = neighbors[i];

    // 				if (tri == best_triangle)
    // 				{
    // 					neighbors[i] = neighbors[neighbors_size - 1];
    // 					adjacency.counts[index]--;
    // 					break;
    // 				}
    // 			}
    // 		}

    // 		// update aggregated meshlet cone data for scoring subsequent triangles
    // 		meshlet_cone_acc.px += triangles[best_triangle].px;
    // 		meshlet_cone_acc.py += triangles[best_triangle].py;
    // 		meshlet_cone_acc.pz += triangles[best_triangle].pz;
    // 		meshlet_cone_acc.nx += triangles[best_triangle].nx;
    // 		meshlet_cone_acc.ny += triangles[best_triangle].ny;
    // 		meshlet_cone_acc.nz += triangles[best_triangle].nz;

    // 		emitted_flags[best_triangle] = 1;
  }

  // 	if (meshlet.triangle_count)
  // 	{
  // 		finishMeshlet(meshlet, meshlet_triangles);

  // 		meshlets[meshlet_offset++] = meshlet;
  // 	}

  // 	assert(meshlet_offset <= meshopt_buildMeshletsBound(index_count, max_vertices, max_triangles));
  // 	return meshlet_offset;
}

pub trait SpaceSearchAcceleration {
  fn build<V>(indices: &[u32], vertices: &[V]) -> Self;
  fn search_nearest(&self, position: Vec3<f32>) -> u32;
}

// static void finishMeshlet(meshopt_Meshlet& meshlet, unsigned char* meshlet_triangles)
// {
// 	size_t offset = meshlet.triangle_offset + meshlet.triangle_count * 3;

// 	// fill 4b padding with 0
// 	while (offset & 3)
// 		meshlet_triangles[offset++] = 0;
// }

// static bool appendMeshlet(meshopt_Meshlet& meshlet, unsigned int a, unsigned int b, unsigned int
// c, unsigned char* used, meshopt_Meshlet* meshlets, unsigned int* meshlet_vertices, unsigned char*
// meshlet_triangles, size_t meshlet_offset, size_t max_vertices, size_t max_triangles) {
// 	unsigned char& av = used[a];
// 	unsigned char& bv = used[b];
// 	unsigned char& cv = used[c];

// 	bool result = false;

// 	unsigned int used_extra = (av == 0xff) + (bv == 0xff) + (cv == 0xff);

// 	if (meshlet.vertex_count + used_extra > max_vertices || meshlet.triangle_count >= max_triangles)
// 	{
// 		meshlets[meshlet_offset] = meshlet;

// 		for (size_t j = 0; j < meshlet.vertex_count; ++j)
// 			used[meshlet_vertices[meshlet.vertex_offset + j]] = 0xff;

// 		finishMeshlet(meshlet, meshlet_triangles);

// 		meshlet.vertex_offset += meshlet.vertex_count;
// 		meshlet.triangle_offset += (meshlet.triangle_count * 3 + 3) & ~3; // 4b padding
// 		meshlet.vertex_count = 0;
// 		meshlet.triangle_count = 0;

// 		result = true;
// 	}

// 	if (av == 0xff)
// 	{
// 		av = (unsigned char)meshlet.vertex_count;
// 		meshlet_vertices[meshlet.vertex_offset + meshlet.vertex_count++] = a;
// 	}

// 	if (bv == 0xff)
// 	{
// 		bv = (unsigned char)meshlet.vertex_count;
// 		meshlet_vertices[meshlet.vertex_offset + meshlet.vertex_count++] = b;
// 	}

// 	if (cv == 0xff)
// 	{
// 		cv = (unsigned char)meshlet.vertex_count;
// 		meshlet_vertices[meshlet.vertex_offset + meshlet.vertex_count++] = c;
// 	}

// 	meshlet_triangles[meshlet.triangle_offset + meshlet.triangle_count * 3 + 0] = av;
// 	meshlet_triangles[meshlet.triangle_offset + meshlet.triangle_count * 3 + 1] = bv;
// 	meshlet_triangles[meshlet.triangle_offset + meshlet.triangle_count * 3 + 2] = cv;
// 	meshlet.triangle_count++;

// 	return result;
// }

fn getNeighborTriangle(
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

      let mut score = 0.;

      // caller selects one of two scoring functions: geometrical (based on meshlet cone) or
      // topological (based on remaining triangles)
      if let Some(meshlet_cone) = meshlet_cone {
        let tri_cone = &triangles[triangle];

        let distance2 = (tri_cone.p - meshlet_cone.p).length2();
        let spread = tri_cone.n.dot(meshlet_cone.n);

        score = get_meshlet_score(distance2, spread, cone_weight, meshlet_expected_radius);
      } else {
        // each live_triangles entry is >= 1 since it includes the current triangle we're processing
        score = (live_triangles[a] + live_triangles[b] + live_triangles[c] - 3) as f32;
      }

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
