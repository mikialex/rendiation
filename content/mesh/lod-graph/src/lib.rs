use std::ops::Range;

use rendiation_algebra::*;
use rendiation_geometry::Triangle;
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_segmentation::{SegmentResult, SegmentationSource, SegmentationStrategy};

pub struct MeshLODGraphSimplificationConfig {
  pub target_tri_num: u32,
  pub tri_num_limit: u32,
  pub target_error: f32,
}

pub struct MeshLODGraphSimplificationResult {
  pub mesh: MeshBufferSource,
  pub error: f32,
}

pub trait SimplificationImplProvider {
  fn simplify(
    mesh: MeshBufferSource,
    locked_edges: Vec<u32>,
    config: MeshLODGraphSimplificationConfig,
  ) -> MeshLODGraphSimplificationResult;
}

#[derive(Clone, Copy)]
pub struct OffsetSize {
  pub offset: u32,
  pub size: u32,
}

impl OffsetSize {
  pub fn into_range(self) -> Range<usize> {
    self.offset as usize..(self.offset + self.size) as usize
  }
}

impl From<Range<u32>> for OffsetSize {
  fn from(value: Range<u32>) -> Self {
    Self {
      offset: value.start,
      size: value.len() as u32,
    }
  }
}

#[derive(Clone, Copy)]
pub struct MeshletGroup {
  pub meshlets: OffsetSize,
}

#[derive(Clone, Copy)]
pub struct Meshlet {
  pub group_index: u32,
  pub index_range: OffsetSize,
}

#[derive(Clone, Copy)]
pub struct MeshLodGraphBuildConfig {
  pub meshlet_size: u32,
}

pub struct MeshLODGraph {
  pub build_config: MeshLodGraphBuildConfig,
  pub levels: Vec<MeshLODGraphLevel>,
}

impl MeshLODGraph {
  pub fn build_from_mesh(mesh: MeshBufferSource, config: MeshLodGraphBuildConfig) -> Self {
    let mut last_level = MeshLODGraphLevel::build_base_from_mesh(mesh, &config);
    let mut levels = Vec::new();

    // if the last level is single group single meshlet, we will have nothing to do and finish build
    while last_level.meshlets.len() == 1 {
      let new_last_level = MeshLODGraphLevel::build_from_finer_level(&last_level, &config);
      let last_last_level = std::mem::replace(&mut last_level, new_last_level);
      levels.push(last_last_level);
    }

    levels.push(last_level);

    Self {
      build_config: config,
      levels,
    }
  }
}

pub struct MeshLODGraphLevel {
  pub groups: Vec<MeshletGroup>,
  pub meshlets: Vec<Meshlet>,
  pub mesh: MeshBufferSource,
  /// for each group, map the previous level meshlet range.
  pub fine_level_meshlet_mapping: Option<Vec<FinerLevelMapping>>,
}

pub struct FinerLevelMapping {
  pub meshlets: OffsetSize,
  pub simplification_error: f32,
}

pub struct MeshBufferSource {
  pub indices: Vec<u32>,
  pub vertices: Vec<CommonVertex>,
}

type TrianglePrimitive = Triangle<Vec3<f32>>;

impl SegmentationSource for MeshBufferSource {
  type Item = TrianglePrimitive;

  fn count(&self) -> u32 {
    (self.indices.len() / 3) as u32
  }

  fn get_item(&self, index: u32) -> Option<Self::Item> {
    let idx = (index as usize) * 3;
    Some(Triangle::new(
      self.vertices[idx].position,
      self.vertices[idx].position,
      self.vertices[idx].position,
    ))
  }
}

impl MeshLODGraphLevel {
  pub fn build_from_finer_level(
    previous_level: &MeshLODGraphLevel,
    config: &MeshLodGraphBuildConfig,
  ) -> Self {
    let mut all_simplified_indices: Vec<u32> =
      Vec::with_capacity(previous_level.mesh.indices.len());
    let mut all_simplified_vertices: Vec<CommonVertex> =
      Vec::with_capacity(previous_level.mesh.vertices.len());
    let mut all_meshlets: Vec<Meshlet> = Vec::with_capacity(previous_level.meshlets.len());
    let mut simplification_error: Vec<f32> = Vec::with_capacity(previous_level.meshlets.len());

    previous_level.groups.iter().for_each(|group| {
      // let simplification_source = group.meshlets.into_range()

      let simplified: MeshLODGraphSimplificationResult = todo!();

      let (meshlets, simplified_mesh) = build_meshlets_from_triangles(simplified.mesh);
      all_simplified_indices.extend(simplified_mesh.indices);
      all_simplified_vertices.extend(simplified_mesh.vertices);
      simplification_error.push(simplified.error);

      all_meshlets.extend(meshlets);
    });

    let mesh = MeshBufferSource {
      indices: all_simplified_indices,
      vertices: all_simplified_vertices,
    };

    let (groups, meshlets) = build_groups_from_meshlets(all_meshlets);

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
      fine_level_meshlet_mapping: Some(fine_level_meshlet_mapping),
    }
  }
  pub fn build_base_from_mesh(mesh: MeshBufferSource, config: &MeshLodGraphBuildConfig) -> Self {
    let (meshlets, mesh) = build_meshlets_from_triangles(mesh);

    let (groups, meshlets) = build_groups_from_meshlets(meshlets);

    Self {
      groups,
      meshlets,
      mesh,
      fine_level_meshlet_mapping: None,
    }
  }
}

struct DefaultSegmentationImpl;
impl<T: SegmentationSource> SegmentationStrategy<T> for DefaultSegmentationImpl {
  fn segmentation(&mut self, input: &T) -> SegmentResult {
    todo!()
  }
}

struct MeshletSegmentationSource<'a>(&'a [Meshlet]);
impl<'a> SegmentationSource for MeshletSegmentationSource<'a> {
  type Item = Meshlet;

  fn count(&self) -> u32 {
    self.0.len() as u32
  }

  fn get_item(&self, index: u32) -> Option<Self::Item> {
    self.0.get(index as usize).cloned()
  }
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

/// reorder indices by given triangle order
fn reorder_meshlet(indices: &[Meshlet], reorder: &[u32]) -> Vec<Meshlet> {
  reorder.iter().map(|idx| indices[*idx as usize]).collect()
}

fn build_meshlets_from_triangles(triangles: MeshBufferSource) -> (Vec<Meshlet>, MeshBufferSource) {
  let triangle_segmentation = DefaultSegmentationImpl.segmentation(&triangles);

  let meshlets: Vec<_> = triangle_segmentation
    .ranges
    .into_iter()
    .map(|v| Meshlet {
      group_index: u32::MAX, // write later
      index_range: v.into(),
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

fn build_groups_from_meshlets(meshlets: Vec<Meshlet>) -> (Vec<MeshletGroup>, Vec<Meshlet>) {
  let meshlet_segmentation =
    DefaultSegmentationImpl.segmentation(&MeshletSegmentationSource(&meshlets));

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

  (groups, meshlets)
}
