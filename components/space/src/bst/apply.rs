use super::{BSTBounding, BSTTree, BinarySpaceTree};
use crate::utils::BuildPrimitive;
use rendiation_algebra::vec3;
use rendiation_geometry::Box3;

pub type BinaryTree = BSTTree<Binary, 2, 1>;
pub type QuadTree = BSTTree<Quad, 4, 2>;
pub type OcTree = BSTTree<Oc, 8, 3>;

pub struct Binary;
pub struct Quad;
pub struct Oc;

impl BSTBounding<3, 8> for Box3 {
  #[allow(clippy::bool_to_int_with_if)]
  fn pre_classify_primitive(&self, p: &BuildPrimitive<Self>) -> usize {
    let dir = self.center() - p.center;
    let mut i: usize = 0;
    i += if dir.x > 0.0 { 0 } else { 1 };
    i += if dir.y > 0.0 { 0 } else { 2 };
    i += if dir.z > 0.0 { 0 } else { 4 };
    i
  }

  #[rustfmt::skip]
  fn compute_sub_space(&self, i: usize) -> Self {
    let center = self.center();
    let (x_min, x_max) = if i & 1 > 0 { (center.x, self.max.x) } else { (self.min.x, center.x) };
    let (y_min, y_max) = if i & 2 > 0 { (center.y, self.max.y) } else { (self.min.y, center.y) };
    let (z_min, z_max) = if i & 4 > 0 { (center.z, self.max.z) } else { (self.min.z, center.z) };
    Box3::new(vec3(x_min, y_min, z_min), vec3(x_max, y_max, z_max))
  }
}

impl BinarySpaceTree<3, 8> for Oc {
  type Bounding = Box3;
}
