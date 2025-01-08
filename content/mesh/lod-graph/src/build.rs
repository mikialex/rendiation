use fast_hash_collection::FastHashSet;

use crate::*;

pub trait MeshLodGraphBuilder {
  fn simplify(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
    locked_edges: &EdgeFinder,
    target_tri_num: u32,
  ) -> MeshLODGraphSimplificationResult;

  fn segment_triangles(&self, input: MeshBufferSource) -> (Vec<Meshlet>, MeshBufferSource);
  fn segment_meshlets(&self, input: &[Meshlet], adj: &MeshletAdjacencyInfo) -> SegmentResult;

  fn build_from_mesh(&self, mesh: MeshBufferSource) -> MeshLODGraph
  where
    Self: Sized,
  {
    let mut last_level = MeshLODGraphLevel::build_base_from_mesh(self, mesh);
    let mut levels = Vec::new();

    // if the last level is single meshlet, we will have nothing to do
    // and finish build
    while last_level.meshlets.len() != 1 {
      let new_last_level = MeshLODGraphLevel::build_from_finer_level(self, &mut last_level);
      let last_last_level = std::mem::replace(&mut last_level, new_last_level);
      levels.push(last_last_level);
    }

    levels.push(last_level);

    MeshLODGraph { levels }
  }
}

pub struct MeshLODGraphSimplificationResult {
  pub mesh: MeshBufferSource,
  pub error: f32,
}

impl MeshLODGraphLevel {
  fn build_from_finer_level(
    builder: &dyn MeshLodGraphBuilder,
    previous_level: &mut MeshLODGraphLevel,
  ) -> Self {
    let mut all_simplified_indices: Vec<u32> =
      Vec::with_capacity(previous_level.mesh.indices.len());
    let mut all_simplified_vertices: Vec<CommonVertex> =
      Vec::with_capacity(previous_level.mesh.vertices.len());
    let mut all_meshlets: Vec<Meshlet> = Vec::with_capacity(previous_level.meshlets.len());
    let mut simplification_error: Vec<f32> = Vec::with_capacity(previous_level.meshlets.len());

    let mut offset = 0;
    let mut ranges: Vec<OffsetSize> = Vec::with_capacity(previous_level.meshlets.len());

    let edges =
      compute_all_meshlet_boundary_edges(&previous_level.meshlets, &previous_level.mesh.indices);
    let meshlet_adjacency = MeshletAdjacencyInfo::build(&edges);

    previous_level
      .groups
      .iter()
      .enumerate()
      .for_each(|(group_idx, group)| {
        // collect all indices in this group, deduplicate adjacent indices between
        // meshlets
        let mut index_range = FastHashSet::default();
        for meshlet in previous_level
          .meshlets
          .get_mut(group.meshlets.into_range())
          .unwrap()
        {
          for idx in previous_level
            .mesh
            .indices
            .get(meshlet.index_range.into_range())
            .unwrap()
          {
            index_range.insert(*idx);
          }
        }
        let index_range: Vec<_> = index_range.drain().collect();

        let locked_edges = compute_locking_edge(&previous_level.groups, group_idx as u32, &edges);

        let simplified = builder.simplify(
          &previous_level.mesh.vertices,
          &index_range,
          &locked_edges,
          index_range.len() as u32 / 2, // remove half of face
        );

        let (meshlets, simplified_mesh) = builder.segment_triangles(simplified.mesh);
        all_simplified_indices.extend(simplified_mesh.indices);
        all_simplified_vertices.extend(simplified_mesh.vertices);
        simplification_error.push(simplified.error);

        all_meshlets.extend(&meshlets);
        let meshlets_len = meshlets.len() as u32;
        ranges.push(OffsetSize {
          offset,
          size: meshlets_len - offset,
        });
        offset += meshlets_len;
      });

    previous_level
      .groups
      .iter_mut()
      .zip(simplification_error.iter())
      .for_each(|(g, err)| g.lod_error_simplify_to_next_level = Some(*err));

    let mesh = MeshBufferSource {
      indices: all_simplified_indices,
      vertices: all_simplified_vertices,
    };

    let (mut groups, mut meshlets, reorder) =
      build_groups_from_meshlets(builder, all_meshlets.clone(), meshlet_adjacency);

    for (group_id, simplified_meshlet_range) in ranges.iter().enumerate() {
      for simplified_meshlet_idx in simplified_meshlet_range.into_range() {
        let simplified_meshlet_idx = reorder[simplified_meshlet_idx];
        let simplified_meshlet = &mut meshlets[simplified_meshlet_idx as usize];
        simplified_meshlet.group_index_in_previous_level = Some(group_id as u32);
      }
    }

    for g in &mut groups {
      let meshlets = meshlets.get(g.meshlets.into_range()).unwrap();
      let mut max_error = 0.;
      for meshlet in meshlets {
        let source_group =
          &previous_level.groups[meshlet.group_index_in_previous_level.unwrap() as usize];
        let error = source_group.max_meshlet_simplification_error;
        max_error = max_error.max(error);
      }
      g.max_meshlet_simplification_error = max_error + g.lod_error_simplify_to_next_level.unwrap();
    }

    Self {
      groups,
      meshlets,
      mesh,
    }
  }

  fn build_base_from_mesh(builder: &dyn MeshLodGraphBuilder, mesh: MeshBufferSource) -> Self {
    let (meshlets, mesh) = builder.segment_triangles(mesh);

    let edges = compute_all_meshlet_boundary_edges(&meshlets, &mesh.indices);
    let meshlet_adjacency = MeshletAdjacencyInfo::build(&edges);
    let (groups, meshlets, _) = build_groups_from_meshlets(builder, meshlets, meshlet_adjacency);

    Self {
      groups,
      meshlets,
      mesh,
    }
  }
}

fn build_groups_from_meshlets(
  builder: &dyn MeshLodGraphBuilder,
  meshlets: Vec<Meshlet>,
  adj: MeshletAdjacencyInfo,
) -> (Vec<MeshletGroup>, Vec<Meshlet>, Vec<u32>) {
  let meshlet_segmentation = builder.segment_meshlets(&meshlets, &adj);

  let groups: Vec<_> = meshlet_segmentation
    .ranges
    .into_iter()
    .map(|v| MeshletGroup {
      meshlets: v.into(),
      lod_error_simplify_to_next_level: None, // write when do simplification to next level
      max_meshlet_simplification_error: 0.,   // no error in source mesh
    })
    .collect();

  let mut meshlets = reorder_meshlet(&meshlets, &meshlet_segmentation.reordered_idx);

  groups.iter().enumerate().for_each(|(i, group)| {
    meshlets
      .get_mut(group.meshlets.into_range())
      .unwrap()
      .iter_mut()
      .for_each(|meshlet| meshlet.group_index = i as u32)
  });

  (groups, meshlets, meshlet_segmentation.reordered_idx)
}

/// reorder indices by given triangle order
fn reorder_meshlet(indices: &[Meshlet], reorder: &[u32]) -> Vec<Meshlet> {
  reorder.iter().map(|idx| indices[*idx as usize]).collect()
}
