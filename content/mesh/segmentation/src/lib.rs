#![feature(array_chunks)]

use std::ops::Range;

use rendiation_algebra::*;
use rendiation_geometry::*;

mod spatial_adjacency_clustering;
use rendiation_mesh_core::AbstractMesh;
pub use spatial_adjacency_clustering::*;

pub trait SegmentationStrategy<T: SegmentationSource> {
  fn segmentation(&mut self, input: &T) -> SegmentResult;
}

pub trait SegmentationSource {
  type Item;

  fn count(&self) -> u32;
  fn get_item(&self, index: u32) -> Option<Self::Item>;
}

pub trait SegmentationSourceAdjacency: SegmentationSource {
  fn build_adjacency(&self) -> impl AdjacencyLookup;
}

pub trait AdjacencyLookup {
  fn iter_adjacent(&self, index: u32) -> impl Iterator<Item = u32> + '_;
  fn remove_adjacent(&mut self, index: u32, adj_to_remove: u32);
}

pub struct SegmentResult {
  pub reordered_idx: Vec<u32>,
  pub ranges: Vec<Range<u32>>,
}

pub struct AbstractMeshAsPrimitiveSource<'a, T>(pub &'a T);
impl<'a, T: AbstractMesh> SegmentationSource for AbstractMeshAsPrimitiveSource<'a, T> {
  type Item = T::Primitive;

  fn count(&self) -> u32 {
    self.0.primitive_count() as u32
  }

  fn get_item(&self, index: u32) -> Option<Self::Item> {
    self.0.primitive_at(index as usize)
  }
}
