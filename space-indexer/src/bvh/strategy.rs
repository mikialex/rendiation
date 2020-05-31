use super::{
  box_from_build_source, node::FlattenBVHNode, BVHBounding, BVHOption, BuildPrimitive,
  FlattenBVHNodeChildInfo,
};
use std::ops::Range;

pub trait BVHBuildStrategy<B: BVHBounding> {
  /// build the bvh tree in given range of primitive source and index.
  /// return the size of tree.
  fn build(
    option: &BVHOption,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode<B>>,
  ) -> usize {
    let (depth, range, split_axis) = {
      let node = nodes.last_mut().unwrap();
      if node.depth == option.max_tree_depth {
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
      (node.depth, range, split_axis)
    };

    let ((left_bbox, left_range), (right_bbox, right_range)) =
      Self::split(range, build_source, index_source);

    let node_index = nodes.len() - 1;

    nodes.push(FlattenBVHNode::new(
      left_bbox,
      left_range,
      nodes.len(),
      depth + 1,
    ));
    let left_count = Self::build(option, build_source, index_source, nodes);

    nodes.push(FlattenBVHNode::new(
      right_bbox,
      right_range,
      nodes.len(),
      depth + 1,
    ));
    let right_count = Self::build(option, build_source, index_source, nodes);

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
    range: Range<usize>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &Vec<usize>,
  ) -> ((B, Range<usize>), (B, Range<usize>));
}

pub struct BalanceTree;

impl<B: BVHBounding> BVHBuildStrategy<B> for BalanceTree {
  fn split(
    range: Range<usize>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &Vec<usize>,
  ) -> ((B, Range<usize>), (B, Range<usize>)) {
    let middle = (range.end - range.start) / 2;
    let left_range = range.start..middle;
    let right_range = middle..range.end;

    let left_bbox = box_from_build_source(&index_source, &build_source, left_range.clone());
    let right_bbox = box_from_build_source(&index_source, &build_source, right_range.clone());

    ((left_bbox, left_range), (right_bbox, right_range))
  }
}
