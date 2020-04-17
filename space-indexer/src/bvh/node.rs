use rendiation_math_entity::{Axis, Box3};
use std::ops::Range;

pub struct FlattenBVHNode {
  pub bbox: Box3,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub self_index: usize,
  pub child: Option<FlattenBVHNodeChildInfo>,
}

impl FlattenBVHNode {
  pub(super) fn new(
    bbox: Box3,
    primitive_range: Range<usize>,
    self_index: usize, 
    depth: usize,
  ) -> Self {
    Self {
      bbox,
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

pub struct FlattenBVHNodeChildInfo {
  pub left_count: usize,
  pub right_count: usize,
  pub split_axis: Axis,
}
