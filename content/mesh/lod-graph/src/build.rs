use fast_hash_collection::FastHashSet;

use crate::*;

impl MeshLODGraph {
  pub fn build_from_mesh(builder: &dyn MeshLodGraphBuilder, mesh: MeshBufferSource) -> Self {
    let mut last_level = MeshLODGraphLevel::build_base_from_mesh(builder, mesh);
    let mut levels = Vec::new();

    // if the last level is single group single meshlet, we will have nothing to do
    // and finish build
    while last_level.meshlets.len() == 1 {
      let new_last_level = MeshLODGraphLevel::build_from_finer_level(builder, &mut last_level);
      let last_last_level = std::mem::replace(&mut last_level, new_last_level);
      levels.push(last_last_level);
    }

    levels.push(last_level);

    Self { levels }
  }
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

        let (meshlets, simplified_mesh) = build_meshlets_from_triangles(builder, simplified.mesh);
        all_simplified_indices.extend(simplified_mesh.indices);
        all_simplified_vertices.extend(simplified_mesh.vertices);
        simplification_error.push(simplified.error);

        all_meshlets.extend(&meshlets);
        let meshlets_len = meshlets.len() as u32;
        ranges.push(OffsetSize {
          offset,
          size: meshlets_len,
        });
        offset += meshlets_len;
      });

    let mesh = MeshBufferSource {
      indices: all_simplified_indices,
      vertices: all_simplified_vertices,
    };

    let (groups, meshlets, reorder) =
      build_groups_from_meshlets(builder, all_meshlets.clone(), meshlet_adjacency);

    // build pervious level's meshlet parents
    let mut parent_meshlets_idx = Vec::with_capacity(meshlets.len());
    for (simplified_meshlet_range, previous_level_group) in
      ranges.iter().zip(previous_level.groups.iter())
    {
      for previous_level_meshlet in previous_level
        .meshlets
        .get_mut(previous_level_group.meshlets.into_range())
        .unwrap()
      {
        let offset = parent_meshlets_idx.len();
        let mut parent_count = 0;
        for simplified_meshlet in simplified_meshlet_range.into_range() {
          let parent_meshlet_idx = reorder[simplified_meshlet];
          parent_meshlets_idx.push(parent_meshlet_idx);
          parent_count += 1;
        }
        previous_level_meshlet.parent_index_range = OffsetSize {
          offset: offset as u32,
          size: parent_count,
        }
        .into();
      }
    }

    let fine_level_meshlet_mapping = previous_level
      .groups
      .iter()
      .zip(simplification_error.iter())
      .map(|(group, &simplification_error)| FinerLevelMapping {
        meshlets: group.meshlets,
        simplification_error,
      })
      .collect();

    Self {
      groups,
      meshlets,
      mesh,
      finer_level_meshlet_mapping: Some(fine_level_meshlet_mapping),
      parent_meshlets_idx,
    }
  }

  fn build_base_from_mesh(builder: &dyn MeshLodGraphBuilder, mesh: MeshBufferSource) -> Self {
    let (meshlets, mesh) = build_meshlets_from_triangles(builder, mesh);

    let edges = compute_all_meshlet_boundary_edges(&meshlets, &mesh.indices);
    let meshlet_adjacency = MeshletAdjacencyInfo::build(&edges);
    let (groups, meshlets, _) = build_groups_from_meshlets(builder, meshlets, meshlet_adjacency);

    Self {
      groups,
      meshlets,
      mesh,
      finer_level_meshlet_mapping: None,
      parent_meshlets_idx: Vec::new(), // set when coarser level build
    }
  }
}

fn build_meshlets_from_triangles(
  builder: &dyn MeshLodGraphBuilder,
  triangles: MeshBufferSource,
) -> (Vec<Meshlet>, MeshBufferSource) {
  let triangle_segmentation = builder.segment_triangles(&triangles);

  let meshlets: Vec<_> = triangle_segmentation
    .ranges
    .into_iter()
    .map(|v| Meshlet {
      group_index: u32::MAX, // write later
      index_range: v.into(),
      parent_index_range: None, // write later when building coarser level
      lod_error: 0.,            // write later
      parent_max_lod_error: 0., // write later
    })
    .collect();

  let indices = reorder_indices(&triangles.indices, &triangle_segmentation.reordered_idx);

  (
    meshlets,
    MeshBufferSource {
      indices,
      vertices: triangles.vertices,
    },
  )
}

/// reorder indices by given triangle order
fn reorder_indices(indices: &[u32], triangle_idx: &[u32]) -> Vec<u32> {
  triangle_idx
    .iter()
    .flat_map(|tri| {
      let idx = *tri as usize * 3;
      [indices[idx], indices[idx + 1], indices[idx + 2]]
    })
    .collect()
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
    .map(|v| MeshletGroup { meshlets: v.into() })
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
