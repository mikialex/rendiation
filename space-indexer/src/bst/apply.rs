use super::{BSTNode, BSTTree, BinarySpaceTree};
use crate::utils::BuildPrimitive;
use rendiation_math::Vec3;
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

  fn compute_sub_space(i: usize, all_bounding: Self::Bounding) -> Self::Bounding {
    let center = all_bounding.center();
    let radius = all_bounding.width() / 2.0;
    let x_dir = if i % 2 < 1 { 1.0 } else { -1.0 };
    let y_dir = if i % 4 < 2 { 1.0 } else { -1.0 };
    let z_dir = if i < 4 { 1.0 } else { -1.0 };

    let child_center = center + radius * Vec3::new(x_dir, y_dir, z_dir);
    Box3::new_cube(child_center, radius)
  }
}
