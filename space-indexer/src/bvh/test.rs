use rendiation_math_entity::Box3;

use crate::utils::*;

use super::{BVHBuildStrategy, BVHOption, BalanceTree, FlattenBVH, SAH};

pub fn bvh_build<S: BVHBuildStrategy<Box3>>(
  boxes: &Vec<Box3>,
  strategy: &mut S,
  option: &BVHOption,
) -> FlattenBVH<Box3> {
  FlattenBVH::new(boxes.iter().map(|&b| b), strategy, option)
}

#[test]
fn test_bvh_build() {
  let boxes = generate_boxes_in_space(10000, 1000., 1.);
  bvh_build(
    &boxes,
    &mut BalanceTree,
    &BVHOption {
      max_tree_depth: 15,
      bin_size: 10,
    },
  );
  bvh_build(
    &boxes,
    &mut SAH::new(4),
    &BVHOption {
      max_tree_depth: 15,
      bin_size: 10,
    },
  );
}
