mod node;
mod strategy;

pub use node::*;
use rendiation_math::Vec3;
use rendiation_math_entity::{Axis, Box3};
use std::cmp::Ordering;
pub use strategy::*;

pub trait FlattenBVHBuildSource {
  fn get_items_count(&self) -> usize;
  fn get_items_bounding_box(&self, item_index: usize) -> Box3;
}

pub struct BVHOption {
  pub max_tree_depth: usize,
  pub bin_size: usize,
}

impl Default for BVHOption {
  fn default() -> Self {
    Self {
      max_tree_depth: 10,
      bin_size: 1,
    }
  }
}

pub struct FlattenBVH {
  nodes: Vec<FlattenBVHNode>,
  sorted_primitive_index: Vec<usize>,
  option: BVHOption,
}

impl FlattenBVH {
  pub fn build<T: BVHBuildStrategy>(source: impl FlattenBVHBuildSource) -> Self {
    let option = BVHOption::default();

    // parepare build source;
    let items_count = source.get_items_count();
    let mut index_list: Vec<usize> = (0..items_count).map(|x| x).collect();
    let primitives: Vec<BuildPrimitive> = (0..items_count)
      .map(|x| BuildPrimitive::new(source.get_items_bounding_box(x)))
      .collect();

    // prepare root
    let mut nodes = Vec::new();
    nodes.push(FlattenBVHNode::new(&primitives, &index_list, 0..items_count, 0));

    // build
    T::build(&option, &primitives, &mut index_list, &mut nodes);

    Self {
      nodes,
      sorted_primitive_index: index_list,
      option,
    }
  }
}

pub struct BuildPrimitive {
  bbox: Box3,
  center: Vec3<f32>,
}

impl BuildPrimitive {
  fn new(bbox: Box3) -> Self {
    Self {
      bbox,
      center: bbox.center(),
    }
  }

  fn compare_center(&self, axis: Axis, other: &BuildPrimitive) -> Ordering {
    match axis {
      Axis::X => self.center.x.partial_cmp(&other.center.x).unwrap(),
      Axis::Y => self.center.y.partial_cmp(&other.center.y).unwrap(),
      Axis::Z => self.center.z.partial_cmp(&other.center.z).unwrap(),
    }
  }
}
