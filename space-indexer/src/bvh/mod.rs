mod apply;
mod node;
mod strategy;
mod traverse;

pub mod test;

pub use node::*;
use std::iter::FromIterator;
pub use strategy::*;

use crate::utils::{
  bounding_from_build_source, BuildPrimitive, CenterAblePrimitive, TreeBuildOption,
};

pub trait BVHBounding: Sized + Copy + FromIterator<Self> + CenterAblePrimitive {
  type AxisType: Copy;
  fn get_partition_axis(&self) -> Self::AxisType;
}

pub struct FlattenBVH<B: BVHBounding> {
  pub nodes: Vec<FlattenBVHNode<B>>,
  pub sorted_primitive_index: Vec<usize>,
}

impl<B: BVHBounding> FlattenBVH<B> {
  pub fn new<S: BVHBuildStrategy<B>>(
    source: impl Iterator<Item = B>,
    strategy: &mut S,
    option: &TreeBuildOption,
  ) -> Self {
    // prepare build source;
    let (mut index_list, primitives) = source
      .enumerate()
      .map(|(i, b)| (i, BuildPrimitive::new(b)))
      .unzip();

    // prepare root
    let root_bbox = bounding_from_build_source(&index_list, &primitives, 0..index_list.len());

    let mut nodes = Vec::new();
    nodes.push(FlattenBVHNode::new(root_bbox, 0..index_list.len(), 0));

    // build
    strategy.build(&option, &primitives, &mut index_list, &mut nodes, 0);

    Self {
      nodes,
      sorted_primitive_index: index_list,
    }
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }
}
