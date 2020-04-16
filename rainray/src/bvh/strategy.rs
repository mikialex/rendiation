use super::{node::FlattenBVHNode, BVHOption, BuildPrimitive};

pub trait BVHBuildStrategy {
  fn build(
    option: &BVHOption,
    build_source: &Vec<BuildPrimitive>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode>,
    node_index: usize,
  );
}

pub struct BalanceTree;

impl BVHBuildStrategy for BalanceTree {
  fn build(
    option: &BVHOption,
    build_source: &Vec<BuildPrimitive>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode>,
    node_index: usize,
  ) -> usize {
    let node = &nodes[node_index];
    if node.depth == option.max_tree_depth{
      return;
    }

    let range = node.primitive_range.clone();
    if range.len() <= option.bin_size {
      return;
    }

    let ranged_index = index_source.get_mut(range).unwrap();
    let split_axis = node.bbox.longest_axis();

    ranged_index.sort_unstable_by(|a, b| {
      let bp_a = &build_source[*a];
      let bp_b = &build_source[*b];
      bp_a.compare_center(split_axis, bp_b)
    });

    
  }
}
