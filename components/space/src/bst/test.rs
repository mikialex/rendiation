#[test]
pub fn test_bvh_build() {
  use super::OcTree;
  use crate::utils::*;
  let boxes = generate_boxes_in_space(32, 1000., 1.);
  let _ = OcTree::new(boxes.iter().cloned(), &TreeBuildOption::default());
}
