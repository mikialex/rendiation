use super::{
  bounding_from_build_source, node::FlattenBVHNode, BVHBounding, BVHOption, BuildPrimitive,
  FlattenBVHNodeChildInfo,
};
use std::ops::Range;

pub trait BVHBuildStrategy<B: BVHBounding> {
  /// build the bvh tree in given range of primitive source and index.
  /// return the size of tree.
  fn build(
    &mut self,
    option: &BVHOption,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode<B>>,
  ) -> usize {
    let (depth, split_axis, node) = {
      let node = nodes.last().unwrap();
      let depth = node.depth;
      if depth == option.max_tree_depth {
        return 1;
      }

      let range = node.primitive_range.clone();
      if range.len() <= option.bin_size {
        return 1;
      }

      let split_axis = B::get_partition_axis(node, build_source, index_source);
      let ranged_index = index_source.get_mut(range.clone()).unwrap();

      ranged_index.sort_unstable_by(|a, b| {
        let bp_a = &build_source[*a];
        let bp_b = &build_source[*b];
        bp_a.compare_center(split_axis, bp_b)
      });
      (depth, split_axis, node)
    };

    let ((left_bbox, left_range), (right_bbox, right_range)) =
      Self::split(self, split_axis, node, build_source, index_source);

    let node_index = nodes.len() - 1;

    nodes.push(FlattenBVHNode::new(
      left_bbox,
      left_range,
      nodes.len(),
      depth + 1,
    ));
    let left_count = Self::build(self, option, build_source, index_source, nodes);

    nodes.push(FlattenBVHNode::new(
      right_bbox,
      right_range,
      nodes.len(),
      depth + 1,
    ));
    let right_count = Self::build(self, option, build_source, index_source, nodes);

    let node = &mut nodes[node_index];
    node.child = Some(FlattenBVHNodeChildInfo {
      left_count,
      right_count,
      split_axis,
    });

    left_count + right_count
  }

  /// different strategy has different split method;
  /// given a range, and return the left, right partition;
  ///
  /// the reason why return bounding is to avoid extra bounding calculation:
  /// partition decision maybe has already computed bounding;
  fn split(
    &mut self,
    split: B::AxisType,
    parent_node: &FlattenBVHNode<B>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &Vec<usize>,
  ) -> ((B, Range<usize>), (B, Range<usize>));
}

pub struct BalanceTree;

impl<B: BVHBounding> BVHBuildStrategy<B> for BalanceTree {
  fn split(
    &mut self,
    _: B::AxisType,
    parent_node: &FlattenBVHNode<B>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &Vec<usize>,
  ) -> ((B, Range<usize>), (B, Range<usize>)) {
    let range = parent_node.primitive_range;
    let middle = (range.end - range.start) / 2;
    let left_range = range.start..middle;
    let right_range = middle..range.end;

    let left_bbox = bounding_from_build_source(&index_source, &build_source, left_range.clone());
    let right_bbox = bounding_from_build_source(&index_source, &build_source, right_range.clone());

    ((left_bbox, left_range), (right_bbox, right_range))
  }
}

pub trait BVHSAHBounding: BVHBounding {
  // type AxisType: Copy;
  // type CenterType;
  // fn get_center(&self) -> Self::CenterType;
  // fn from_groups(iter: impl Iterator<Item = Self>) -> Self;
  // fn get_partition_axis(
  //   node: &FlattenBVHNode<Self>,
  //   build_source: &Vec<BuildPrimitive<Self>>,
  //   index_source: &Vec<usize>,
  // ) -> Self::AxisType;
  // fn compare(
  //   self_primitive: &BuildPrimitive<Self>,
  //   axis: Self::AxisType,
  //   other_primitive: &BuildPrimitive<Self>,
  // ) -> Ordering;
}

pub struct SAH<B: BVHBounding> {
  pub pre_partition_check_count: usize,
  checks: Vec<SAHPrePartitionCache<B>>
}

impl<B: BVHBounding> SAH<B>{
  pub fn new(pre_partition_check_count: usize)-> Self{
    Self{
      pre_partition_check_count,
      checks: Vec::with_capacity(pre_partition_check_count)
    }
  }

  pub fn get_split()
}

struct SAHPrePartitionCache<B: BVHBounding>{
  bounding: B,
  primitive_count: usize,
}

impl<B: BVHBounding> BVHBuildStrategy<B> for SAH<B> {
  fn split(
    &mut self,
    split: B::AxisType,
    parent_node: &FlattenBVHNode<B>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &Vec<usize>,
  ) -> ((B, Range<usize>), (B, Range<usize>)) {
    todo!()
    let extent = 
  }
}
