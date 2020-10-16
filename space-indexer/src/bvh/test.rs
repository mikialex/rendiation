use rendiation_math_entity::Box3;

use crate::utils::*;

use super::{BVHOption, BalanceTree, FlattenBVH};

pub fn bvh_build(boxes: &Vec<Box3>) -> FlattenBVH<Box3> {
  FlattenBVH::new(
    boxes.iter().map(|&b| b),
    &mut BalanceTree,
    &BVHOption {
      max_tree_depth: 15,
      bin_size: 10,
    },
  )
}

#[test]
fn test_bvh_build() {
  let boxes = generate_boxes_in_space(10000, 1000., 1.);
  bvh_build(&boxes);
}
