use super::{node::FlattenBVHNode, BVHOption, BuildPrimitive, FlattenBVHNodeChildInfo};
use std::ops::Range;

pub trait BVHBuildStrategy {
  fn build(
    option: &BVHOption,
    build_source: &Vec<BuildPrimitive>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode>,
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

      let ranged_index = index_source.get_mut(range.clone()).unwrap();
      let split_axis = node.bbox.longest_axis();

      ranged_index.sort_unstable_by(|a, b| {
        let bp_a = &build_source[*a];
        let bp_b = &build_source[*b];
        bp_a.compare_center(split_axis, bp_b)
      });
      (node.depth, range, split_axis)
    };

    let (left_node, right_node) = Self::split(range, build_source, index_source, depth);

    let node_index = nodes.len();

    nodes.push(left_node);
    let left_count = Self::build(option, build_source, index_source, nodes);
    nodes.push(right_node);
    let right_count = Self::build(option, build_source, index_source, nodes);

    let node = &mut nodes[node_index];
    node.child = Some(FlattenBVHNodeChildInfo {
      left_count,
      right_count,
      split_axis,
    });

    left_count + right_count
  }

  fn split(
    range: Range<usize>,
    build_source: &Vec<BuildPrimitive>,
    index_source: &Vec<usize>,
    depth: usize,
  ) -> (FlattenBVHNode, FlattenBVHNode);
}

pub struct BalanceTree;

impl BVHBuildStrategy for BalanceTree {
  fn split(
    range: Range<usize>,
    build_source: &Vec<BuildPrimitive>,
    index_source: &Vec<usize>,
    depth: usize,
  ) -> (FlattenBVHNode, FlattenBVHNode) {
    let middle = (range.end - range.start) / 2;
    let left_range = range.start..middle;
    let right_range = middle..range.end;
    (
      FlattenBVHNode::new(build_source, index_source, left_range, depth + 1),
      FlattenBVHNode::new(build_source, index_source, right_range, depth + 1),
    )
  }
}
