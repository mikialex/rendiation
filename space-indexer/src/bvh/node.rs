use super::{BVHBounding, FlattenBVH};
use std::ops::Range;

pub struct FlattenBVHNode<B: BVHBounding> {
  pub bounding: B,
  pub primitive_range: Range<usize>,
  pub self_index: usize,
  pub child: Option<FlattenBVHNodeChildInfo<B>>,
}

impl<B: BVHBounding> FlattenBVHNode<B> {
  pub fn iter_primitive<'a>(&'a self, tree: &'a FlattenBVH<B>) -> impl Iterator<Item = &'a usize> {
    tree
      .sorted_primitive_index
      .get(self.primitive_range.clone())
      .unwrap()
      .iter()
  }
}

pub struct FlattenBVHNodeChildInfo<B: BVHBounding> {
  pub left_count: usize,
  pub split_axis: B::AxisType,
}

impl<B: BVHBounding> FlattenBVHNode<B> {
  pub(super) fn new(bounding: B, primitive_range: Range<usize>, self_index: usize) -> Self {
    Self {
      bounding,
      primitive_range,
      self_index,
      child: None,
    }
  }

  pub fn is_leaf(&self) -> bool {
    self.child.is_none()
  }

  pub fn left_child_offset(&self) -> Option<usize> {
    self.child.as_ref().map(|_| self.self_index + 1)
  }

  pub fn right_child_offset(&self) -> Option<usize> {
    self
      .child
      .as_ref()
      .map(|c| self.self_index + c.left_count + 1)
  }
}
