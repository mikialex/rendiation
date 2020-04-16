use super::BuildPrimitive;
use rendiation_math_entity::{Axis, Box3};
use std::ops::Range;

pub struct FlattenBVHNode {
  pub bbox: Box3,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub child: Option<FlattenBVHNodeChildInfo>,
}

impl FlattenBVHNode {
  pub(super) fn new(
    build_source: &Vec<BuildPrimitive>,
    index_source: &Vec<usize>,
    range: Range<usize>,
    depth: usize,
  ) -> Self {
    let primitive_range = range.clone();
    let ranged_index_source = index_source.get(range).unwrap();
    let bbox = Box3::from_boxes(
      ranged_index_source
        .iter()
        .map(|index| build_source[*index].bbox),
    );
    Self {
      bbox,
      primitive_range,
      depth,
      child: None,
    }
  }
}

pub struct FlattenBVHNodeChildInfo {
  pub left_count: usize,
  pub right_count: usize,
  pub split_axis: Axis,
}
