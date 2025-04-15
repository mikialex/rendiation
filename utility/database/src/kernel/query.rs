use crate::*;

pub struct IterableComponentReadView<T> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentReadViewUntyped,
  pub phantom: PhantomData<T>,
}

impl<T> Clone for IterableComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: CValue> Query for IterableComponentReadView<T> {
  type Key = u32;
  type Value = T;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, T)> + '_ {
    self.ecg.iter_entity_idx().map(|id| unsafe {
      (
        id.alloc_index(),
        self
          .read_view
          .get_without_generation_check(id.alloc_index())
          .map(|v| (*(v as *const T)).clone())
          .unwrap_unchecked(), /* as we iterated from the correct index set,
                                * this unwrap should be safe */
      )
    })
  }

  fn access(&self, key: &u32) -> Option<T> {
    self
      .read_view
      .get_without_generation_check(*key)
      .map(|v| unsafe { &*(v as *const T) })
      .cloned()
  }
}

pub struct IterableComponentReadViewChecked<T> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentReadViewUntyped,
  pub phantom: PhantomData<T>,
}

impl<T> Clone for IterableComponentReadViewChecked<T> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: CValue> Query for IterableComponentReadViewChecked<T> {
  type Key = RawEntityHandle;
  type Value = T;
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, T)> + '_ {
    self.ecg.iter_entity_idx().map(|id| unsafe {
      (
        id,
        self
          .read_view
          .get_without_generation_check(id.index())
          .map(|v| (*(v as *const T)).clone())
          .unwrap_unchecked(), // ditto
      )
    })
  }

  fn access(&self, key: &RawEntityHandle) -> Option<T> {
    self
      .read_view
      .get(*key)
      .map(|v| unsafe { &*(v as *const T) })
      .cloned()
  }
}
