use std::ops::Range;
use super::BVHBounding;

pub struct FlattenBVHNode<B: BVHBounding> {
  pub bounding: B,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub self_index: usize,
  pub child: Option<FlattenBVHNodeChildInfo<B>>,
}

pub struct FlattenBVHNodeChildInfo<B: BVHBounding> {
  pub left_count: usize,
  pub right_count: usize,
  pub split_axis: B::AxisType,
}

impl<B: BVHBounding> FlattenBVHNode<B> {
  pub(super) fn new(
    bounding: B,
    primitive_range: Range<usize>,
    self_index: usize, 
    depth: usize,
  ) -> Self {
    Self {
      bounding,
      primitive_range,
      depth,
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
    self.child.as_ref().map(|c| self.self_index + c.left_count + 1)
  }
}
