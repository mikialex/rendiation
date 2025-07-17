#![feature(array_chunks)]

use std::ops::Range;

use fast_hash_collection::FastHashSet;
use rendiation_algebra::*;
use rendiation_geometry::Sphere;
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_segmentation::SegmentResult;

mod build;
pub use build::*;
mod meshlet_adjacency;
use meshlet_adjacency::*;
mod util;
use facet::*;
use serde::*;
pub use util::*;
mod builder_impl;
pub use builder_impl::*;

#[derive(Clone, Serialize, Deserialize, Facet)]
pub struct MeshLODGraph {
  pub levels: Vec<MeshLODGraphLevel>,
}

#[derive(Clone, Serialize, Deserialize, Facet)]
pub struct MeshLODGraphLevel {
  pub groups: Vec<MeshletGroup>,
  pub meshlets: Vec<Meshlet>,
  /// the index is based on level it self, not the mesh.
  #[facet(opaque)]
  pub mesh: MeshBufferSource,
}

#[derive(Clone, Copy, Serialize, Deserialize, Facet)]
pub struct MeshletGroup {
  pub meshlets: OffsetSize,
  /// the current meshlet simplification error, used for debug
  pub lod_error_simplify_to_next_level: f32,
  /// monotonically increasing, used in rendering
  pub max_meshlet_simplification_error_among_meshlet_in_their_parent_group: f32,
  /// monotonically increasing, used in rendering
  pub union_meshlet_bounding_among_meshlet_in_their_parent_group: Sphere,
}

#[derive(Clone, Copy, Serialize, Deserialize, Facet)]
pub struct Meshlet {
  pub group_index: u32,
  pub group_index_in_previous_level: u32,
  pub index_range: OffsetSize,
  pub bounding_in_local: Sphere,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MeshBufferSource {
  pub indices: Vec<u32>,
  pub vertices: Vec<CommonVertex>,
}
