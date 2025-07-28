use crate::{spatial_adjacency_clustering::connectivity::TriangleAdjacency, *};

pub struct MeshletBuildSeeding {
  seed_triangles: Vec<u32>,
  corner: Vec3<f32>,
}

/// We keep a limited number of seed triangles
const MESHLET_MAX_SEEDS: usize = 256;
/// Add a few triangles per finished meshlet
const MESHLET_ADD_SEEDS: usize = 4;

impl MeshletBuildSeeding {
  pub fn new(triangle_position_iter: impl IntoIterator<Item = Vec3<f32>> + Clone) -> (Self, u32) {
    // find a specific corner of the mesh to use as a starting point for meshlet flow
    let corner = triangle_position_iter
      .clone()
      .into_iter()
      .fold(Vec3::splat(f32::MAX), |current_corner, tri_position| {
        current_corner.zip(tri_position, |c, p| c.min(p))
      });

    // initial seed triangle is the one closest to the corner
    let mut initial_seed = u32::MAX;
    let mut distance_sq = f32::MAX;
    for (i, p) in triangle_position_iter.into_iter().enumerate() {
      let d = p.distance2_to(corner);
      if initial_seed == u32::MAX || d < distance_sq {
        distance_sq = d;
        initial_seed = i as u32;
      }
    }

    let sys = Self {
      seed_triangles: Default::default(),
      corner,
    };

    (sys, initial_seed)
  }

  pub fn prune(&mut self, tri_emitted_flags: &[bool]) {
    // only keep un emitted triangles
    // todo, retain keeps order, is that required?
    self
      .seed_triangles
      .retain(|&index| !tri_emitted_flags[index as usize]);
  }

  pub fn append(
    &mut self,
    meshlet_vertices: &[u32],
    indices: &[u32],
    adj: &TriangleAdjacency,
    triangle_position: impl Fn(u32) -> Vec3<f32>,
  ) {
    self
      .seed_triangles
      .truncate(MESHLET_MAX_SEEDS - MESHLET_ADD_SEEDS);

    let mut best_seeds = [u32::MAX; MESHLET_ADD_SEEDS];
    let mut best_live = [u32::MAX; MESHLET_ADD_SEEDS];
    let mut best_score = [f32::MAX; MESHLET_ADD_SEEDS];

    for index in meshlet_vertices {
      let index = *index;
      let mut best_neighbor = u32::MAX;
      let mut best_neighbor_live = u32::MAX;

      // find the neighbor with the smallest live metric
      let live_triangles = adj.vertex_referenced_face_counts();
      for tri_idx in adj.iter_adjacency_faces(index) {
        let tri_idx = tri_idx as usize;
        let a = indices[tri_idx * 3] as usize;
        let b = indices[tri_idx * 3 + 1] as usize;
        let c = indices[tri_idx * 3 + 2] as usize;

        let live = live_triangles[a] + live_triangles[b] + live_triangles[c];

        if live < best_neighbor_live {
          best_neighbor = tri_idx as u32;
          best_neighbor_live = live;
        }
      }

      // add the neighbor to the list of seeds; the list is unsorted and the replacement criteria is approximate
      if best_neighbor == u32::MAX {
        continue;
      }

      let best_neighbor_score = triangle_position(best_neighbor).distance2_to(self.corner);

      for j in 0..MESHLET_ADD_SEEDS {
        // non-strict comparison reduces the number of duplicate seeds (triangles adjacent to multiple vertices)
        if best_neighbor_live < best_live[j]
          || (best_neighbor_live == best_live[j] && best_neighbor_score <= best_score[j])
        {
          best_seeds[j] = best_neighbor;
          best_live[j] = best_neighbor_live;
          best_score[j] = best_neighbor_score;
          break;
        }
      }
    }

    for seed in best_seeds {
      if seed != u32::MAX {
        self.seed_triangles.push(seed);
      }
    }
  }

  pub fn select_best(
    &mut self,
    indices: &[u32],
    live_triangles: &[u32],
    triangle_position: impl Fn(u32) -> Vec3<f32>,
  ) -> u32 {
    let mut best_seed = u32::MAX;
    let mut best_live = u32::MAX;
    let mut best_score = f32::MAX;

    for seed in &self.seed_triangles {
      let tri_idx = *seed as usize;
      let a = indices[tri_idx * 3] as usize;
      let b = indices[tri_idx * 3 + 1] as usize;
      let c = indices[tri_idx * 3 + 2] as usize;
      let live = live_triangles[a] + live_triangles[b] + live_triangles[c];
      let score = triangle_position(*seed).distance2_to(self.corner);

      if live < best_live || (live == best_live && score < best_score) {
        best_seed = *seed;
        best_live = live;
        best_score = score;
      }
    }

    best_seed
  }
}
