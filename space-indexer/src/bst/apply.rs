use super::{BSTNode, BSTTree, BinarySpaceTree};
use crate::utils::BuildPrimitive;
use rendiation_math_entity::Box3;

pub type BinaryTree = BSTTree<Binary, 2>;
pub type QuadTree = BSTTree<Quad, 4>;
pub type OcTree = BSTTree<Oc, 8>;

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

  fn classify_primitive(
    node: &BSTNode<Self, 8>,
    p: &BuildPrimitive<Self::Bounding>,
  ) -> Option<usize> {
    todo!()
  }

  fn get_sub_space(index: usize, all_bounding: Self::Bounding) -> Self::Bounding {
    todo!()
  }
}
