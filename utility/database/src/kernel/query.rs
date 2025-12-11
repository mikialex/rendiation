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
      let idx = id.alloc_index();
      (idx, self.access_ref(&idx).unwrap_unchecked().clone())
    })
  }

  #[inline]
  fn access(&self, key: &u32) -> Option<T> {
    self.access_ref(key).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.read_view.allocator.is_empty()
  }
}

impl<T: CValue> DynValueRefQuery for IterableComponentReadView<T> {
  #[inline]
  fn access_ref(&self, key: &Self::Key) -> Option<&Self::Value> {
    self
      .read_view
      .get_without_generation_check(*key)
      .map(|v| unsafe { &*(v as *const T) })
  }
}

pub struct IterableComponentReadViewChecked<T> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentReadViewUntyped,
  pub phantom: PhantomData<T>,
}

impl<T> IterableComponentReadViewChecked<T> {
  #[inline]
  pub fn read_ref(&self, key: RawEntityHandle) -> Option<&T> {
    self
      .read_view
      .get(key)
      .map(|v| unsafe { &*(v as *const T) })
  }
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
      // as we iterated from the correct index set,
      // this unwrap should be safe
      (id, self.access_ref(&id).unwrap_unchecked().clone())
    })
  }

  #[inline]
  fn access(&self, key: &RawEntityHandle) -> Option<T> {
    self.read_ref(*key).cloned()
  }

  fn has_item_hint(&self) -> bool {
    !self.read_view.allocator.is_empty()
  }
}

impl<T: CValue> DynValueRefQuery for IterableComponentReadViewChecked<T> {
  #[inline]
  fn access_ref(&self, key: &Self::Key) -> Option<&Self::Value> {
    self.read_ref(*key)
  }
}
