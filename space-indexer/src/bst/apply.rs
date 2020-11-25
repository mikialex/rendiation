use super::{BSTBounding, BSTTree, BinarySpaceTree};
use crate::utils::BuildPrimitive;
use rendiation_math::Vec3;
use rendiation_math_entity::Box3;

pub type BinaryTree = BSTTree<Binary, 2, 1>;
pub type QuadTree = BSTTree<Quad, 4, 2>;
pub type OcTree = BSTTree<Oc, 8, 3>;

pub struct Binary;
pub struct Quad;
pub struct Oc;

impl BSTBounding<3, 8> for Box3 {
  fn pre_classify_primitive(&self, p: &BuildPrimitive<Self>) -> usize {
    let dir = self.center() - p.center;
    let mut i: usize = 0;
    i += if dir.x > 0.0 { 0 } else { 1 };
    i += if dir.y > 0.0 { 0 } else { 2 };
    i += if dir.z > 0.0 { 0 } else { 4 };
    i
  }

  fn compute_sub_space(&self, i: usize) -> Self {
    let center = self.center();
    let half_size = self.half_size();
    let x_dir = if i % 2 < 1 { 1.0 } else { -1.0 };
    let y_dir = if i % 4 < 2 { 1.0 } else { -1.0 };
    let z_dir = if i < 4 { 1.0 } else { -1.0 };

    let child_center = center + half_size * Vec3::new(x_dir, y_dir, z_dir);
    Box3::new_from_center(child_center, half_size * 0.5)
  }
}

impl BinarySpaceTree<3, 8> for Oc {
  type Bounding = Box3;
}
