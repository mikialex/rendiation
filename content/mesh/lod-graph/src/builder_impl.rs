use fast_hash_collection::FastHashMap;
use rendiation_mesh_segmentation::{build_meshlets, build_meshlets_bound, ClusteringConfig};
use rendiation_mesh_simplification::{simplify_by_edge_collapse, EdgeCollapseConfig};

use crate::*;

pub struct DefaultMeshLODBuilder {
  // any config goes here
}

impl MeshLodGraphBuilder for DefaultMeshLODBuilder {
  fn simplify(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
    locked_edges: &EdgeFinder,
    target_tri_num: u32,
  ) -> MeshLODGraphSimplificationResult {
    let mut vertex_lock = vec![false; vertices.len()];
    for edge in locked_edges.0.iter() {
      vertex_lock[edge.0 as usize] = true;
      vertex_lock[edge.1 as usize] = true;
    }

    if DEBUG_LOG {
      let lock_count: usize = vertex_lock.iter().filter(|v| **v).count();
      println!(
        "simplify lock ratio: {}",
        lock_count as f32 / vertices.len() as f32
      );
    }

    let mut simplified_indices = vec![0; indices.len()];
    let result = simplify_by_edge_collapse(
      &mut simplified_indices,
      indices,
      vertices,
      Some(&vertex_lock),
      EdgeCollapseConfig {
        target_index_count: target_tri_num as usize * 3,
        target_error: f32::INFINITY, // disable error limit
        lock_border: false, /* border should be able to be simplified unless it's locked by our config */
      },
    );
    let simplified_indices = simplified_indices
      .get(0..result.result_count)
      .unwrap()
      .to_vec();

    // todo, improve
    let simplified_indices = simplified_indices
      .array_chunks::<3>()
      .filter(|&[a, b, c]| a != b && b != c && c != a)
      .flatten()
      .copied()
      .collect::<Vec<_>>();

    if DEBUG_LOG {
      let ratio = simplified_indices.len() as f32 / indices.len() as f32;
      if ratio > 0.5 {
        println!("warning! simplify ratio not meet requirement: {ratio}",);
      }
    }

    MeshLODGraphSimplificationResult {
      simplified_indices,
      error: result.result_error,
    }
  }

  fn segment_triangles(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
  ) -> (Vec<Meshlet>, Vec<u32>) {
    let config = ClusteringConfig {
      max_vertices: 64,
      max_triangles: 124, // NVidia-recommended 126, rounded down to a multiple of 4
      cone_weight: 0.0,
    };

    let max_meshlets = build_meshlets_bound(indices.len(), &config);
    let mut meshlets = vec![rendiation_mesh_segmentation::Meshlet::default(); max_meshlets];

    let mut meshlet_vertices = vec![0; max_meshlets * config.max_vertices as usize];
    let mut meshlet_triangles = vec![0; max_meshlets * config.max_triangles as usize * 3];

    let count = build_meshlets::<_, rendiation_mesh_segmentation::BVHSpaceSearchAcceleration>(
      &config,
      indices,
      vertices,
      &mut meshlets,
      &mut meshlet_vertices,
      &mut meshlet_triangles,
    );

    let mut indices = Vec::with_capacity(indices.len());
    let mut ranges = Vec::with_capacity(meshlets.len());
    let mut start = 0;

    for meshlet in meshlets.get(0..count).unwrap() {
      let tri_range = meshlet.triangle_offset as usize
        ..(meshlet.triangle_offset + meshlet.triangle_count * 3) as usize;
      let tri = meshlet_triangles
        .get(tri_range)
        .unwrap()
        .array_chunks::<3>();
      for [a, b, c] in tri {
        indices.push(meshlet_vertices[*a as usize]);
        indices.push(meshlet_vertices[*b as usize]);
        indices.push(meshlet_vertices[*c as usize]);
      }

      ranges.push(OffsetSize {
        offset: start,
        size: meshlet.triangle_count * 3,
      });
      start += meshlet.triangle_count * 3;
    }

    let meshlets = ranges
      .into_iter()
      .map(|range| Meshlet {
        group_index: 0, // write later when do meshlet segmentation
        index_range: range,
        group_index_in_previous_level: u32::MAX, // write later
        bounding_in_local: {
          Sphere::from_points(
            indices[range.into_range()]
              .iter()
              .map(|idx| vertices[*idx as usize].position),
          )
        },
      })
      .collect();

    (meshlets, indices)
  }

  fn segment_meshlets(&self, input: &[Meshlet], adj: &MeshletAdjacencyInfo) -> SegmentResult {
    let mut xadj = Vec::with_capacity(input.len() + 1);
    let mut adjncy = Vec::new();
    let mut adjwgt = Vec::new();
    for (id, _) in input.iter().enumerate() {
      xadj.push(adjncy.len() as i32);
      for ad in adj.iter_adjacency_meshlets(id as u32) {
        adjncy.push(ad as i32);
        let shared_edge_count = 1; // todo;
        adjwgt.push(shared_edge_count);
      }
    }
    xadj.push(adjncy.len() as i32);

    let mut group_per_meshlet = vec![0; input.len()];
    let partition_count = (input.len().div_ceil(4)) as i32;
    metis::Graph::new(1, partition_count, &xadj, &adjncy)
      .unwrap()
      .set_adjwgt(&adjwgt)
      .part_kway(&mut group_per_meshlet)
      .unwrap();

    let mut groups = FastHashMap::default();
    for (i, meshlet_group) in group_per_meshlet.into_iter().enumerate() {
      groups
        .entry(meshlet_group)
        .or_insert(Vec::new())
        .push(i as u32);
    }

    let mut reordered_idx = Vec::with_capacity(input.len());
    let mut ranges = Vec::with_capacity(input.len());
    for (_, meshlet_ids) in groups {
      let start = reordered_idx.len() as u32;
      let size = meshlet_ids.len() as u32;
      ranges.push(start..start + size);
      reordered_idx.extend(meshlet_ids);
    }

    SegmentResult {
      reordered_idx,
      ranges,
    }
  }
}
