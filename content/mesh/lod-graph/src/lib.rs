#![feature(array_chunks)]

use std::ops::Range;

use fast_hash_collection::FastHashSet;
use rendiation_algebra::*;
use rendiation_geometry::{Box3, Triangle};
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_segmentation::{SegmentResult, SegmentationSource};

mod build;
pub use build::*;
mod meshlet_adjacency;
pub use build::*;
use meshlet_adjacency::*;
mod util;
pub use util::*;
mod impl_dependency;
pub use impl_dependency::*;

#[derive(Clone, Copy)]
pub struct MeshletGroup {
  pub meshlets: OffsetSize,
}

#[derive(Clone, Copy)]
pub struct Meshlet {
  pub group_index: u32,
  pub index_range: OffsetSize,
  pub parent_index_range: Option<OffsetSize>,

  pub lod_error: f32,
  /// maximum of all parent(coarser) meshlets' lod error.
  /// this is to make sure each meshlet lod decision will have same result if the have same parent
  /// meshlets(if one decide use finer level, the others should use finer level too)
  pub parent_max_lod_error: f32,
  // /// the bounding box of this meshlet, used to do culling
  // pub bounding_box_self: Box3,
}

#[derive(Clone, Copy)]
pub struct MeshLodGraphBuildConfig {
  pub meshlet_size: u32,
}

pub struct MeshLODGraph {
  pub build_config: MeshLodGraphBuildConfig,
  pub levels: Vec<MeshLODGraphLevel>,
}

pub struct MeshLODGraphLevel {
  pub groups: Vec<MeshletGroup>,
  pub meshlets: Vec<Meshlet>,
  pub parent_meshlets_idx: Vec<u32>,
  pub mesh: MeshBufferSource,
  /// for each group, map the previous level meshlet range and simplification error.
  pub finer_level_meshlet_mapping: Option<Vec<FinerLevelMapping>>,
}

pub struct FinerLevelMapping {
  pub meshlets: OffsetSize,
  pub simplification_error: f32,
}

pub struct MeshBufferSource {
  pub indices: Vec<u32>,
  pub vertices: Vec<CommonVertex>,
}
