use super::{BSTNode, BinarySpaceTree};
use crate::utils::BuildPrimitive;
use rendiation_math_entity::Box3;

pub struct Binary;
pub struct Quad;
pub struct Oc;

// impl BinarySpaceTree<4> for Quad {
//   type Bounding = Rectangle;
// }

impl BinarySpaceTree<8> for Oc {
  type Bounding = Box3;
  fn create_outer_bounding(
    build_source: &Vec<BuildPrimitive<Self::Bounding>>,
    index_source: &Vec<usize>,
  ) -> Self::Bounding {
    todo!()
  }

  fn check_primitive_should_in_which_partition(
    primitive: &BuildPrimitive<Self::Bounding>,
  ) -> Option<usize> {
    todo!()
  }

  fn get_sub_space(index: usize, all_bounding: Self::Bounding) -> Self::Bounding {
    todo!()
  }
}
