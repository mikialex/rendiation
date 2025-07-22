#![feature(array_chunks)]

use std::{fmt::Debug, ops::Range};

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
use rendiation_mesh_simplification::*;
use serde::*;
pub use util::*;
mod builder_impl;
pub use builder_impl::*;

const DEBUG_LOG: bool = true;

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

impl MeshLODGraphLevel {
  pub fn print_debug(&self) {
    println!(
      "level info: meshlet count: {}, group_count: {}, indices_len: {}, vertex_len: {}",
      self.meshlets.len(),
      self.groups.len(),
      self.mesh.indices.len(),
      self.mesh.vertices.len()
    );
  }
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

impl MeshBufferSource {
  pub fn remap_vertex(self) -> Self {
    let total_indices = self.indices.len();
    let mut remap = vec![0; total_indices];

    let total_vertices = generate_vertex_remap(&mut remap, Some(&self.indices), &self.vertices);

    let mut new_vertices = vec![CommonVertex::default(); total_vertices];
    let mut new_indices = vec![0; total_indices];

    remap_vertex_buffer(&mut new_vertices, &self.vertices, &remap);
    remap_index_buffer(&mut new_indices, Some(&self.indices), total_indices, &remap);
    Self {
      indices: new_indices,
      vertices: new_vertices,
    }
  }
}
