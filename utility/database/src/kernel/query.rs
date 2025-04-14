use crate::*;

pub struct IterableComponentReadView<T: ComponentSemantic> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentReadView<T>,
}

impl<T: ComponentSemantic> Clone for IterableComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
    }
  }
}

impl<T: ComponentSemantic<Data: CValue>> Query for IterableComponentReadView<T> {
  type Key = u32;
  type Value = T::Data;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, T::Data)> + '_ {
    self.ecg.iter_entity_idx().map(|id| unsafe {
      (
        id.alloc_index(),
        self.read_view.get_by_untyped_handle(id).cloned().unwrap(),
      )
    })
  }

  fn access(&self, key: &u32) -> Option<T::Data> {
    self.read_view.get_without_generation_check(*key).cloned()
  }
}

pub struct IterableComponentReadViewChecked<T: ComponentSemantic> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentReadView<T>,
}

impl<T: ComponentSemantic> Clone for IterableComponentReadViewChecked<T> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
    }
  }
}

impl<T: ComponentSemantic<Data: CValue>> Query for IterableComponentReadViewChecked<T> {
  type Key = RawEntityHandle;
  type Value = T::Data;
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, T::Data)> + '_ {
    self.ecg.iter_entity_idx().map(|id| {
      (
        id,
        self
          .read_view
          .get_value_without_generation_check(id.index())
          .unwrap(),
      )
    })
  }

  fn access(&self, key: &RawEntityHandle) -> Option<T::Data> {
    // todo, this is risky, we can not guarantee the handle type is valid
    unsafe { self.read_view.get_by_untyped_handle(*key).cloned() }
  }
}
