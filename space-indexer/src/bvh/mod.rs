mod node;
mod strategy;
mod traverse;

pub use node::*;
use rendiation_math::Vec3;
use rendiation_math_entity::{Axis, Box3};
use std::{cmp::Ordering, ops::Range};
pub use strategy::*;

// input data protocol
pub trait FlattenBVHBuildSource<B: BVHBounding> {
  fn get_items_count(&self) -> usize;
  fn get_items_bounding_box(&self, item_index: usize) -> B;
}

pub trait BVHBounding: Sized + Copy {
  type PartitionMarker: Copy;
  fn get_center(&self) -> Vec3<f32>;
  fn from_groups(iter: impl Iterator<Item = Self>) -> Self;
  fn get_partition_axis(&self) -> Self::PartitionMarker;
  fn compare(
    self_primitive: &BuildPrimitive<Self>,
    axis: Self::PartitionMarker,
    other_primitive: &BuildPrimitive<Self>,
  ) -> Ordering;
}

pub struct BuildPrimitive<B: BVHBounding> {
  bbox: B,
  center: Vec3<f32>,
}

impl<B: BVHBounding> BuildPrimitive<B> {
  fn new(bbox: B) -> Self {
    Self {
      bbox,
      center: bbox.get_center(),
    }
  }

  fn compare_center(&self, axis: B::PartitionMarker, other: &BuildPrimitive<B>) -> Ordering {
    B::compare(self, axis, &other)
  }
}

impl BVHBounding for Box3 {
  type PartitionMarker = Axis;
  fn get_center(&self) -> Vec3<f32> {
    self.center()
  }
  fn from_groups(iter: impl Iterator<Item = Self>) -> Self {
    Self::from_boxes(iter)
  }
  fn get_partition_axis(&self) -> Self::PartitionMarker {
    self.longest_axis().0
  }

  fn compare(
    self_p: &BuildPrimitive<Self>,
    axis: Self::PartitionMarker,
    other: &BuildPrimitive<Self>,
  ) -> Ordering {
    match axis {
      Axis::X => self_p.center.x.partial_cmp(&other.center.x).unwrap(),
      Axis::Y => self_p.center.y.partial_cmp(&other.center.y).unwrap(),
      Axis::Z => self_p.center.z.partial_cmp(&other.center.z).unwrap(),
    }
  }
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

pub struct FlattenBVH<B: BVHBounding> {
  nodes: Vec<FlattenBVHNode<B>>,
  sorted_primitive_index: Vec<usize>,
  option: BVHOption,
}

impl<B: BVHBounding> FlattenBVH<B> {
  pub fn new<T: BVHBuildStrategy<B>>(source: impl FlattenBVHBuildSource<B>) -> Self {
    let option = BVHOption::default();

    // prepare build source;
    let items_count = source.get_items_count();
    let mut index_list: Vec<usize> = (0..items_count).map(|x| x).collect();
    let primitives: Vec<BuildPrimitive<B>> = (0..items_count)
      .map(|x| BuildPrimitive::new(source.get_items_bounding_box(x)))
      .collect();

    // prepare root
    let root_bbox = box_from_build_source(&index_list, &primitives, 0..items_count);

    let mut nodes = Vec::new();
    nodes.push(FlattenBVHNode::new(root_bbox, 0..items_count, 0, 0));

    // build
    T::build(&option, &primitives, &mut index_list, &mut nodes);

    Self {
      nodes,
      sorted_primitive_index: index_list,
      option,
    }
  }

  pub fn option(&self) -> &BVHOption {
    &self.option
  }

  pub fn sorted_primitive_index(&self) -> &Vec<usize> {
    &self.sorted_primitive_index
  }
}

fn box_from_build_source<B: BVHBounding>(
  index_list: &Vec<usize>,
  primitives: &Vec<BuildPrimitive<B>>,
  range: Range<usize>,
) -> B {
  B::from_groups(
    index_list
      .get(range.clone())
      .unwrap()
      .iter()
      .map(|index| primitives[*index].bbox),
  )
}
