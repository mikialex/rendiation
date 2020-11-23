use super::{BSTNode, BSTTree, BinarySpaceTree};
use crate::utils::BuildPrimitive;
use rendiation_math::Vec3;
use rendiation_math_entity::{Box3, ContainAble};

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

  fn classify_primitive(
    parent_box: &Self::Bounding,
    children_bounding: &[Self::Bounding],
    p: &BuildPrimitive<Self::Bounding>,
  ) -> Option<usize> {
    let dir = parent_box.center() - p.center;
    let mut i: usize = 0;
    i += if dir.x > 0.0 { 0 } else { 1 };
    i += if dir.y > 0.0 { 0 } else { 2 };
    i += if dir.z > 0.0 { 0 } else { 4 };

    if children_bounding[i].contains(&p.bounding) {
      Some(i)
    } else {
      None
    }
  }

  fn compute_sub_space(i: usize, all_bounding: Self::Bounding) -> Self::Bounding {
    let center = all_bounding.center();
    let half_size = all_bounding.half_size();
    let x_dir = if i % 2 < 1 { 1.0 } else { -1.0 };
    let y_dir = if i % 4 < 2 { 1.0 } else { -1.0 };
    let z_dir = if i < 4 { 1.0 } else { -1.0 };

    let child_center = center + half_size * Vec3::new(x_dir, y_dir, z_dir);
    Box3::new_from_center(child_center, half_size * 0.5)
  }
}
