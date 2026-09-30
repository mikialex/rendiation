use crate::*;

pub struct IterableComponentReadView<T> {
  pub table: ArcTable,
  pub read_view: ComponentReadViewUntyped,
  pub phantom: PhantomData<T>,
}

impl<T> Clone for IterableComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      table: self.table.clone(),
      read_view: self.read_view.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: CValue> Query for IterableComponentReadView<T> {
  type Key = u32;
  type Value = T;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, T)> + '_ {
    self.table.iter_entity_idx().map(|id| unsafe {
      // as we iterated from the living index set, the slot check can be skipped
      let idx = id.alloc_index();
      let value = &*(self.read_view.data.get(idx) as *const T);
      (idx, value.clone())
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
  pub table: ArcTable,
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
      table: self.table.clone(),
      read_view: self.read_view.clone(),
      phantom: PhantomData,
    }
  }
}

impl<T: CValue> Query for IterableComponentReadViewChecked<T> {
  type Key = RawEntityHandle;
  type Value = T;
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, T)> + '_ {
    self.table.iter_entity_idx().map(|id| unsafe {
      // as we iterated from the living index set, the generation and slot check can be skipped
      let value = &*(self.read_view.data.get(id.alloc_index()) as *const T);
      (id, value.clone())
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
