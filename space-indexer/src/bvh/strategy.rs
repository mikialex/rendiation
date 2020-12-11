use crate::utils::{bounding_from_build_source, TreeBuildOption};

use super::{node::FlattenBVHNode, BVHBounding, BuildPrimitive, FlattenBVHNodeChildInfo};
use std::{iter::FromIterator, ops::Range};

pub trait BVHBuildStrategy<B: BVHBounding> {
  /// build the bvh tree in given range of primitive source and index.
  /// return the size of tree.
  fn build(
    &mut self,
    option: &TreeBuildOption,
    build_source: &[BuildPrimitive<B>],
    index_source: &mut Vec<usize>,
    nodes: &mut Vec<FlattenBVHNode<B>>,
    depth: usize,
  ) -> usize {
    let node = nodes.last().unwrap();
    if !option.should_continue(node.primitive_range.len(), depth) {
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

    left_count + right_count + 1
  }

  /// different strategy has different split method;
  /// given a range, and return the left, right partition and split decision;
  fn split(
    &mut self,
    parent_node: &FlattenBVHNode<B>,
    build_source: &[BuildPrimitive<B>],
    index_source: &mut Vec<usize>,
  ) -> ((B, Range<usize>), B::AxisType, (B, Range<usize>));
}

pub struct BalanceTree;

pub trait BalanceTreeBounding: BVHBounding {
  fn median_partition_at_axis(
    range: Range<usize>,
    build_source: &[BuildPrimitive<Self>],
    index_source: &mut Vec<usize>,
    axis: Self::AxisType,
  );
}

impl<B: BalanceTreeBounding> BVHBuildStrategy<B> for BalanceTree {
  fn split(
    &mut self,
    parent_node: &FlattenBVHNode<B>,
    build_source: &[BuildPrimitive<B>],
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

pub trait SAHBounding: BalanceTreeBounding + Default {
  fn get_surface_heuristic(&self) -> f32;
  fn get_unit_from_center_by_axis(center: &Self::Center, axis: Self::AxisType) -> f32;
  fn get_unit_range_by_axis(&self, split: Self::AxisType) -> Range<f32>;
  fn empty() -> Self;
  fn union(&mut self, other: Self);
}

pub struct SAH<B: SAHBounding> {
  pre_partition: Vec<SAHPrePartitionCache<B>>,
  partition_decision: Vec<(SAHPartitionGroup<B>, SAHPartitionGroup<B>, f32)>,
}

impl<B: SAHBounding> SAH<B> {
  pub fn new(pre_partition_check_count: usize) -> Self {
    Self {
      pre_partition: vec![SAHPrePartitionCache::default(); pre_partition_check_count],
      partition_decision: vec![
        (
          SAHPartitionGroup::default(),
          SAHPartitionGroup::default(),
          0.0
        );
        pre_partition_check_count - 1
      ],
    }
  }

  /// Check if all primitive partitioned in one bucket.
  /// This case occurred when primitive's bounding all overlapped nearly together.
  fn is_partition_degenerate(&self) -> bool {
    self
      .pre_partition
      .iter()
      .filter(|p| p.primitive_bucket.is_empty())
      .count()
      == self.partition_count() - 1
  }

  fn partition_count(&self) -> usize {
    self.pre_partition.len()
  }
  fn reset(&mut self) {
    self.pre_partition.iter_mut().for_each(|p| p.reset());
  }
}

#[derive(Clone, Debug)]
struct SAHPrePartitionCache<B: SAHBounding> {
  bounding: B,
  primitive_bucket: Vec<usize>,
}

impl<B: SAHBounding> Default for SAHPrePartitionCache<B> {
  fn default() -> Self {
    Self {
      primitive_bucket: Vec::new(),
      bounding: B::default(),
    }
  }
}

#[derive(Clone, Debug)]
struct SAHPartitionGroup<B: SAHBounding> {
  bounding: B,
  primitive_count: usize,
}
impl<B: SAHBounding> SAHPartitionGroup<B> {
  fn cost(&self) -> f32 {
    self.bounding.get_surface_heuristic() * self.primitive_count as f32
  }
}

impl<B: SAHBounding> Default for SAHPartitionGroup<B> {
  fn default() -> Self {
    Self {
      bounding: B::empty(),
      primitive_count: 0,
    }
  }
}

impl<'a, B: SAHBounding> FromIterator<&'a SAHPrePartitionCache<B>> for SAHPartitionGroup<B> {
  fn from_iter<I: IntoIterator<Item = &'a SAHPrePartitionCache<B>>>(items: I) -> Self {
    let mut primitive_count = 0;
    let bounding = items
      .into_iter()
      .map(|p| {
        primitive_count += p.primitive_bucket.len();
        p.bounding
      })
      .collect();
    Self {
      bounding,
      primitive_count,
    }
  }
}

impl<B: SAHBounding> SAHPrePartitionCache<B> {
  fn reset(&mut self) {
    self.primitive_bucket.clear();
    self.bounding = B::empty();
  }
  fn set_primitive(&mut self, p: &BuildPrimitive<B>, index: usize) {
    self.bounding.union(p.bounding);
    self.primitive_bucket.push(index);
  }
}

impl<B: SAHBounding> BVHBuildStrategy<B> for SAH<B> {
  fn split(
    &mut self,
    parent_node: &FlattenBVHNode<B>,
    build_source: &[BuildPrimitive<B>],
    index_source: &mut Vec<usize>,
  ) -> ((B, Range<usize>), B::AxisType, (B, Range<usize>)) {
    // step 1, update pre_partition_check_cache
    let range = parent_node.primitive_range.clone();
    self.reset();
    let axis = parent_node.bounding.get_partition_axis();
    let axis_range = parent_node.bounding.get_unit_range_by_axis(axis);
    let step = (axis_range.end - axis_range.start) / self.partition_count() as f32;

    index_source
      .get(range.clone())
      .unwrap()
      .iter()
      .map(|&index| (&build_source[index], index))
      .for_each(|(p, index)| {
        let axis_value = B::get_unit_from_center_by_axis(&p.center, axis);
        let mut which_partition = ((axis_value - axis_range.start) / step).floor() as usize;
        // edge case
        if which_partition == self.pre_partition.len() {
          which_partition -= 1;
        }
        self.pre_partition[which_partition].set_primitive(p, index)
      });

    if self.is_partition_degenerate() {
      let mut fallback = BalanceTree;
      return fallback.split(parent_node, build_source, index_source);
    }

    // step 2, find best partition;
    let pre_partition = &self.pre_partition;
    self
      .partition_decision
      .iter_mut()
      .enumerate()
      .for_each(|(i, (l, r, cost))| {
        let (left, right) = pre_partition.as_slice().split_at(i + 1);
        *l = left.iter().collect();
        *r = right.iter().collect();
        *cost = l.cost() + r.cost();
      });

    let mut best = 0;
    let mut best_cost = std::f32::INFINITY;
    self
      .partition_decision
      .iter()
      .enumerate()
      .for_each(|(i, &(_, _, cost))| {
        if cost < best_cost {
          best_cost = cost;
          best = i;
        }
      });
    let best_pair = &self.partition_decision[best];

    // step3. update and return
    // todo use unsafe for perf
    let mut ptr = range.start;
    for i in 0..self.partition_count() {
      let p = &self.pre_partition[i];
      p.primitive_bucket.iter().for_each(|i| {
        index_source[ptr] = *i;
        ptr += 1;
      })
    }

    (
      (
        best_pair.0.bounding,
        range.start..(range.start + best_pair.0.primitive_count),
      ),
      axis,
      (
        best_pair.1.bounding,
        (range.start + best_pair.0.primitive_count)..range.end,
      ),
    )
  }
}
