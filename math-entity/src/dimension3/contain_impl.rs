use crate::{Box3, ContainAble};

impl ContainAble<Box3> for Box3 {
  fn contains(&self, items_to_contain: &Box3) -> bool {
    todo!()
  }
}
