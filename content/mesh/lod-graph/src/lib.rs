#![feature(array_chunks)]

use std::ops::Range;

use fast_hash_collection::FastHashSet;
use rendiation_algebra::*;
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_segmentation::SegmentResult;

mod build;
pub use build::*;
mod meshlet_adjacency;
use meshlet_adjacency::*;
mod util;
pub use util::*;
mod builder_impl;
pub use builder_impl::*;

#[derive(Clone, Copy)]
pub struct MeshletGroup {
  pub meshlets: OffsetSize,
  pub lod_error_simplify_to_next_level: Option<f32>,
  pub max_meshlet_simplification_error: f32,
}

#[derive(Clone, Copy)]
pub struct Meshlet {
  pub group_index: u32,
  pub index_range: OffsetSize,
  pub group_index_in_previous_level: Option<u32>,
}

pub struct MeshLODGraph {
  pub levels: Vec<MeshLODGraphLevel>,
}

pub struct MeshLODGraphLevel {
  pub groups: Vec<MeshletGroup>,
  pub meshlets: Vec<Meshlet>,
  pub mesh: MeshBufferSource,
}

pub struct FinerLevelMapping {
  pub meshlets: OffsetSize,
  pub simplification_error: f32,
}

pub struct MeshBufferSource {
  pub indices: Vec<u32>,
  pub vertices: Vec<CommonVertex>,
}
