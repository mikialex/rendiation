use rendiation_geometry::Box3;

use crate::utils::TreeBuildOption;

use super::{BVHBuildStrategy, FlattenBVH};

pub fn bvh_build<S: BVHBuildStrategy<Box3>>(
  boxes: &[Box3],
  strategy: &mut S,
  option: &TreeBuildOption,
) -> FlattenBVH<Box3> {
  FlattenBVH::new(boxes.iter().cloned(), strategy, option)
}

#[test]
pub fn test_bvh_build() {
  use super::{BalanceTree, SAH};
  use crate::utils::*;
  let boxes = generate_boxes_in_space(32, 1000., 1.);
  bvh_build(
    &boxes,
    &mut BalanceTree,
    &TreeBuildOption {
      max_tree_depth: 15,
      bin_size: 10,
    },
  );
  bvh_build(
    &boxes,
    &mut SAH::new(4),
    &TreeBuildOption {
      max_tree_depth: 15,
      bin_size: 10,
    },
  );
}
