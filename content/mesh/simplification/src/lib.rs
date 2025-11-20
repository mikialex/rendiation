#![feature(iter_array_chunks)]
#![allow(clippy::disallowed_types)] // we have already used custom hasher

use std::collections::{HashMap, hash_map::Entry};

use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;

mod qem;
use qem::*;

mod hasher;
use hasher::*;

mod remap;
use remap::*;

mod edge_collapse;
use edge_collapse::*;

mod sloppy;

mod connectivity;
use connectivity::*;

const INVALID_INDEX: u32 = u32::MAX;

pub use edge_collapse::{EdgeCollapseConfig, simplify_by_edge_collapse};
pub use sloppy::simplify_sloppy;

#[derive(Clone, Copy)]
pub struct SimplificationResult {
  /// the result error rate
  pub result_error: f32,
  /// the number of indices after simplification.
  ///
  ///  The resulting index buffer references vertices from the original vertex buffer.
  /// If the original vertex data isn't required, creating a compact vertex buffer is recommended.
  pub result_count: usize,
}

/// rescale the vertex into unit cube with min(0,0,0)
fn rescale_positions<Vertex>(vertices: &[Vertex]) -> (f32, Vec<Vec3<f32>>)
where
  Vertex: Positioned<Position = Vec3<f32>>,
{
  let bbox: Box3 = vertices.iter().map(|v| v.position()).collect();
  let box_size = bbox.size();
  let extent = box_size.x.max(box_size.y).max(box_size.z);
  let scale = inverse_or_zeroed(extent);

  let positions = vertices
    .iter()
    .map(|v| (v.position() - bbox.min) * scale)
    .collect();

  (extent, positions)
}
