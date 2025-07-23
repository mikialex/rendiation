use crate::*;

pub trait MeshLodGraphBuilder {
  fn simplify(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
    locked_edges: &EdgeFinder,
    target_tri_num: u32,
  ) -> MeshLODGraphSimplificationResult;

  fn segment_triangles(
    &self,
    vertices: &[CommonVertex],
    indices: &[u32],
  ) -> (Vec<Meshlet>, Vec<u32>);
  fn segment_meshlets(&self, input: &[Meshlet], adj: &MeshletAdjacencyInfo) -> SegmentResult;

  fn build_from_mesh(&self, mesh: CommonMeshBuffer) -> MeshLODGraph
  where
    Self: Sized,
  {
    let mut last_level = MeshLODGraphLevel::build_base_from_mesh(self, mesh);

    if DEBUG_LOG {
      last_level.print_debug();
    }

    let mut levels = Vec::new();

    // if the last level is single meshlet, we will have nothing to do
    // and finish build
    while last_level.meshlets.len() != 1 {
      let new_last_level = MeshLODGraphLevel::build_from_finer_level(self, &mut last_level);
      if DEBUG_LOG {
        new_last_level.print_debug();
      }

      let last_last_level = std::mem::replace(&mut last_level, new_last_level);
      levels.push(last_last_level);
    }

    levels.push(last_level);

    MeshLODGraph { levels }
  }
}

pub struct MeshLODGraphSimplificationResult {
  pub simplified_indices: Vec<u32>,
  pub error: f32,
}

impl MeshLODGraphLevel {
  fn build_from_finer_level(
    builder: &dyn MeshLodGraphBuilder,
    previous_level: &mut MeshLODGraphLevel,
  ) -> Self {
    let mut all_simplified_indices: Vec<u32> =
      Vec::with_capacity(previous_level.mesh.indices.len());
    let mut all_meshlets: Vec<Meshlet> = Vec::with_capacity(previous_level.meshlets.len());
    let mut simplification_error: Vec<f32> = Vec::with_capacity(previous_level.meshlets.len());

    let mut offset = 0;
    let mut ranges: Vec<OffsetSize> = Vec::with_capacity(previous_level.meshlets.len());

    let edges =
      compute_all_meshlet_boundary_edges(&previous_level.meshlets, &previous_level.mesh.indices);

    previous_level
      .groups
      .iter()
      .enumerate()
      .for_each(|(group_idx, group)| {
        // combine all indices in this group
        let all_indices_in_group = previous_level
          .meshlets
          .get_mut(group.meshlets.into_range())
          .unwrap()
          .iter()
          .flat_map(|meshlet| {
            previous_level
              .mesh
              .indices
              .get(meshlet.index_range.into_range())
              .unwrap()
              .iter()
              .cloned()
          })
          .collect::<Vec<_>>();

        let locked_edges = compute_locking_edge(&previous_level.groups, group_idx as u32, &edges);

        let simplified = builder.simplify(
          &previous_level.mesh.vertices,
          &all_indices_in_group,
          &locked_edges,
          all_indices_in_group.len() as u32 / 3 / 2, // remove half of face
        );

        let (meshlets, reordered_simplified_indices) = builder.segment_triangles(
          &previous_level.mesh.vertices,
          &simplified.simplified_indices,
        );
        all_simplified_indices.extend(reordered_simplified_indices);
        simplification_error.push(simplified.error);

        all_meshlets.extend(&meshlets);
        let meshlets_len = meshlets.len() as u32;
        ranges.push(OffsetSize {
          offset,
          size: meshlets_len,
        });
        offset += meshlets_len;
      });

    previous_level
      .groups
      .iter_mut()
      .zip(simplification_error.iter())
      .for_each(|(g, err)| {
        g.lod_error_simplify_to_next_level = *err;
        g.max_meshlet_simplification_error_among_meshlet_in_their_parent_group += *err;
      });

    let edges = compute_all_meshlet_boundary_edges(&all_meshlets, &all_simplified_indices);
    let meshlet_adjacency = MeshletAdjacencyInfo::build(&edges);

    let (mut groups, mut reordered_meshlets, reorder) =
      build_groups_from_meshlets(builder, &all_meshlets, meshlet_adjacency, false);

    for (group_id, simplified_meshlet_range) in ranges.iter().enumerate() {
      for simplified_meshlet_idx in simplified_meshlet_range.into_range() {
        let reordered_simplified_meshlet_idx = reorder[simplified_meshlet_idx];
        let simplified_meshlet = &mut reordered_meshlets[reordered_simplified_meshlet_idx as usize];
        simplified_meshlet.group_index_in_previous_level = group_id as u32;
      }
    }

    for g in &mut groups {
      let meshlets = reordered_meshlets.get(g.meshlets.into_range()).unwrap();

      let meshlets_source_groups_in_parent_level = meshlets
        .iter()
        .map(|meshlet| previous_level.groups[meshlet.group_index_in_previous_level as usize]);

      g.max_meshlet_simplification_error_among_meshlet_in_their_parent_group =
        meshlets_source_groups_in_parent_level
          .clone()
          .fold(0., |err, parent_group| {
            err.max(
              parent_group.max_meshlet_simplification_error_among_meshlet_in_their_parent_group,
            )
          });

      // todo, check if this bounding merge is necessary
      g.union_meshlet_bounding_among_meshlet_in_their_parent_group = Sphere::from_spheres(
        meshlets_source_groups_in_parent_level
          .map(|g| g.union_meshlet_bounding_among_meshlet_in_their_parent_group),
      );
    }

    // remove duplicate and not used vertex.
    let (indices, vertices) = create_deduplicated_index_vertex_mesh(
      all_simplified_indices
        .iter()
        .map(|i| previous_level.mesh.vertices[*i as usize]),
    );

    Self {
      groups,
      meshlets: reordered_meshlets,
      mesh: CommonMeshBuffer { indices, vertices },
    }
  }

  fn build_base_from_mesh(builder: &dyn MeshLodGraphBuilder, mesh: CommonMeshBuffer) -> Self {
    let (meshlets, reordered_indices) = builder.segment_triangles(&mesh.vertices, &mesh.indices);

    let edges = compute_all_meshlet_boundary_edges(&meshlets, &reordered_indices);
    let meshlet_adjacency = MeshletAdjacencyInfo::build(&edges);
    let (groups, meshlets, _) =
      build_groups_from_meshlets(builder, &meshlets, meshlet_adjacency, true);

    Self {
      groups,
      meshlets,
      mesh,
    }
  }
}

fn build_groups_from_meshlets(
  builder: &dyn MeshLodGraphBuilder,
  meshlets: &[Meshlet],
  adj: MeshletAdjacencyInfo,
  is_level_0: bool,
) -> (Vec<MeshletGroup>, Vec<Meshlet>, Vec<u32>) {
  let meshlet_segmentation = builder.segment_meshlets(meshlets, &adj);

  let mut meshlets = reorder_meshlet(meshlets, &meshlet_segmentation.reordered_idx);

  let groups: Vec<_> = meshlet_segmentation
    .ranges
    .into_iter()
    .map(|v| MeshletGroup {
      meshlets: v.clone().into(),
      lod_error_simplify_to_next_level: 0., // write later when do simplification to next level
      max_meshlet_simplification_error_among_meshlet_in_their_parent_group: 0., // write later
      union_meshlet_bounding_among_meshlet_in_their_parent_group: if is_level_0 {
        Sphere::from_spheres(
          meshlets[(v.start as usize)..(v.end as usize)]
            .iter()
            .map(|m| m.bounding_in_local),
        )
      } else {
        Sphere::new(Vec3::zero(), 0.) // write later
      },
    })
    .collect();

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
