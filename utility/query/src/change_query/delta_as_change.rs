use crate::*;

#[derive(Clone)]
pub struct DeltaQueryAsChange<T>(pub T);

pub trait IntoDeltaQueryAsChangeExt: Sized {
  fn into_change(self) -> DeltaQueryAsChange<Self> {
    DeltaQueryAsChange(self)
  }
}
impl<T: Query> IntoDeltaQueryAsChangeExt for T {}

impl<T: CValue, Q: Query<Value = ValueChange<T>>> DataChanges for DeltaQueryAsChange<Q> {
  type Key = Q::Key;
  type Value = T;

  fn has_change(&self) -> bool {
    // iter_key_value may have heap allocation, use this to do a pre check
    // todo, we should add this to Query trait to avoid box
    if !self.0.has_item_hint() {
      return false;
    }

    self.0.iter_key_value().next().is_some()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|(k, v)| v.is_removed().then_some(k))
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|v| v.1.new_value().map(|x| (v.0, x.clone())))
  }
}
