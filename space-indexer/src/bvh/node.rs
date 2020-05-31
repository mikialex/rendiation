use std::ops::Range;

pub struct FlattenBVHNode<B, P> {
  pub bounding: B,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub self_index: usize,
  pub child: Option<FlattenBVHNodeChildInfo<P>>,
}

pub struct FlattenBVHNodeChildInfo<P> {
  pub left_count: usize,
  pub right_count: usize,
  pub split_axis: P,
}

impl<B, P> FlattenBVHNode<B, P> {
  pub(super) fn new(
    bbox: B,
    primitive_range: Range<usize>,
    self_index: usize, 
    depth: usize,
  ) -> Self {
    Self {
      bounding: bbox,
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
