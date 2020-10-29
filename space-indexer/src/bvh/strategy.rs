use crate::utils::TreeBuildOption;

use super::{
  bounding_from_build_source, node::FlattenBVHNode, BVHBounding, BuildPrimitive,
  FlattenBVHNodeChildInfo,
};
use std::ops::Range;

pub trait BVHBuildStrategy<B: BVHBounding> {
  /// build the bvh tree in given range of primitive source and index.
  /// return the size of tree.
  fn build(
    &mut self,
    option: &TreeBuildOption,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode<B>>,
    depth: usize,
  ) -> usize {
    let node = nodes.last().unwrap();
    if !option.should_continue(node, depth) {
      return 1;
    }

    let ((left_bbox, left_range), split_axis, (right_bbox, right_range)) =
      Self::split(self, node, build_source, index_source);

    let node_index = nodes.len() - 1;

    nodes.push(FlattenBVHNode::new(left_bbox, left_range, nodes.len()));
    let left_count = Self::build(self, option, build_source, index_source, nodes, depth + 1);

    nodes.push(FlattenBVHNode::new(right_bbox, right_range, nodes.len()));
    let right_count = Self::build(self, option, build_source, index_source, nodes, depth + 1);

    let node = &mut nodes[node_index];

    node.child = Some(FlattenBVHNodeChildInfo {
      left_count,
      split_axis,
    });

    left_count + right_count
  }

  /// different strategy has different split method;
  /// given a range, and return the left, right partition and split decision;
  ///
  fn split(
    &mut self,
    parent_node: &FlattenBVHNode<B>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &mut Vec<usize>,
  ) -> ((B, Range<usize>), B::AxisType, (B, Range<usize>));
}

pub struct BalanceTree;

pub trait BalanceTreeBounding: BVHBounding {
  fn median_partition_at_axis(
    range: Range<usize>,
    build_source: &Vec<BuildPrimitive<Self>>,
    index_source: &mut Vec<usize>,
    axis: Self::AxisType,
  );
}

impl<B: BalanceTreeBounding> BVHBuildStrategy<B> for BalanceTree {
  fn split(
    &mut self,
    parent_node: &FlattenBVHNode<B>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &mut Vec<usize>,
  ) -> ((B, Range<usize>), B::AxisType, (B, Range<usize>)) {
    let axis = parent_node.bounding.get_partition_axis();

    let range = parent_node.primitive_range.clone();
    let middle = (range.end + range.start) / 2;
    let left_range = range.start..middle;
    let right_range = middle..range.end;

    B::median_partition_at_axis(range, build_source, index_source, axis);

    let left_bbox = bounding_from_build_source(&index_source, &build_source, left_range.clone());
    let right_bbox = bounding_from_build_source(&index_source, &build_source, right_range.clone());

    ((left_bbox, left_range), axis, (right_bbox, right_range))
  }
}

pub trait SAHBounding: BVHBounding + Default {
  fn get_surface_heuristic(&self) -> f32;
  fn get_unit_from_center_by_axis(center: &Self::Center, axis: Self::AxisType) -> f32;
  fn get_unit_range_by_axis(&self, split: Self::AxisType) -> Range<f32>;
}

pub struct SAH<B: SAHBounding> {
  pub pre_partition_check_count: usize,
  pre_partition: Vec<SAHPrePartitionCache<B>>,
}

impl<B: SAHBounding> SAH<B> {
  pub fn new(pre_partition_check_count: usize) -> Self {
    Self {
      pre_partition_check_count,
      pre_partition: vec![SAHPrePartitionCache::default(); pre_partition_check_count],
    }
  }
  fn group_box(&self, range: Range<usize>) -> B {
    self
      .pre_partition
      .get(range)
      .unwrap()
      .iter()
      .map(|p| p.bounding)
      .collect()
  }
}

#[derive(Clone, Debug)]
struct SAHPrePartitionCache<B: SAHBounding> {
  bounding: B,
  primitive_range: Range<usize>,
}

impl<B: SAHBounding> Default for SAHPrePartitionCache<B> {
  fn default() -> Self {
    Self {
      bounding: B::default(),
      primitive_range: 0..0,
    }
  }
}

impl<B: SAHBounding> SAHPrePartitionCache<B> {
  fn cost(&self) -> f32 {
    self.bounding.get_surface_heuristic() * self.primitive_range.clone().count() as f32
  }
}

impl<B: SAHBounding> BVHBuildStrategy<B> for SAH<B> {
  fn split(
    &mut self,
    parent_node: &FlattenBVHNode<B>,
    build_source: &Vec<BuildPrimitive<B>>,
    index_source: &mut Vec<usize>,
  ) -> ((B, Range<usize>), B::AxisType, (B, Range<usize>)) {
    // step 1, update pre_partition_check_cache
    let axis = parent_node.bounding.get_partition_axis();
    let partition_count = self.pre_partition_check_count;
    let range = parent_node.bounding.get_unit_range_by_axis(axis);
    let range_len = range.end - range.start;
    let step = range_len / partition_count as f32;

    let primitive_range = &parent_node.primitive_range;
    let mut primitive_checked_offset = primitive_range.start;

    self
      .pre_partition
      .iter_mut()
      .enumerate()
      .for_each(|(i, p)| {
        if i == partition_count - 1 {
          p.bounding = bounding_from_build_source(
            &index_source,
            &build_source,
            primitive_checked_offset..primitive_range.end,
          );
          p.primitive_range = primitive_checked_offset..primitive_range.end;
          return;
        }

        let extent_largest = range.start + step * (i + 1) as f32;
        let mut exceed = false;
        let start_primitive_range = primitive_checked_offset;

        while !exceed && primitive_checked_offset < primitive_range.end {
          let build_primitive = index_source[primitive_checked_offset];
          exceed = B::get_unit_from_center_by_axis(&build_source[build_primitive].center, axis)
            > extent_largest;
          primitive_checked_offset += 1;
        }

        if exceed {
          primitive_checked_offset -= 1;
        }

        p.bounding = bounding_from_build_source(
          &index_source,
          &build_source,
          start_primitive_range..primitive_checked_offset,
        );
        p.primitive_range = start_primitive_range..primitive_checked_offset;
      });

    // step 2, find best partition;

    let mut left = self.pre_partition[0].clone(); // just need a initial value;
    let mut right = left.clone(); // ditto
    right.primitive_range = primitive_range.clone();
    left.primitive_range.end = left.primitive_range.start;

    let mut best_cost = std::f32::MAX;
    let mut best_left = left.clone(); // ditto
    let mut best_right = left.clone(); // ditto

    for partition in 0..self.pre_partition_check_count - 1 {
      let add_part = &self.pre_partition[partition];
      let move_count = add_part.primitive_range.clone().count();
      left.primitive_range.end += move_count;
      right.primitive_range.start += move_count;
      left.bounding = self.group_box(0..partition + 1);
      right.bounding = self.group_box(partition + 1..self.pre_partition_check_count);

      let new_cost = left.cost() + right.cost();
      if new_cost < best_cost {
        best_cost = new_cost;
        best_left = left.clone();
        best_right = right.clone();
      }
    }

    (
      (best_left.bounding, best_left.primitive_range),
      axis,
      (best_right.bounding, best_right.primitive_range),
    )
  }
}
