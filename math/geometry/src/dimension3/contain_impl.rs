use rendiation_algebra::Scalar;

use crate::{Box3, ContainAble};

impl<T: Scalar> ContainAble<T, Box3<T>, 3> for Box3<T> {
  fn contains(&self, box3: &Box3<T>) -> bool {
    self.min.x <= box3.min.x
      && self.min.y <= box3.min.y
      && self.min.z <= box3.min.z
      && self.max.x >= box3.max.x
      && self.max.y >= box3.max.y
      && self.max.z >= box3.max.z
  }
}
