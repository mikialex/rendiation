use crate::*;

pub struct ComponentCollection<C> {
  phantom: PhantomData<C>,
  inner: ComponentCollectionUntyped,
}

impl<C> Clone for ComponentCollection<C> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom.clone(),
      inner: self.inner.clone(),
    }
  }
}

impl<C: ComponentSemantic> ComponentCollection<C> {
  pub fn read(&self) -> ComponentReadView<C> {
    todo!()
  }

  pub fn read_foreign_key(&self) -> ForeignKeyReadView<C>
  where
    C: ForeignKeySemantic,
  {
    todo!()
  }

  pub fn write(&self) -> ComponentWriteView<C> {
    todo!()
  }
}

impl ComponentCollectionUntyped {
  pub unsafe fn into_typed<C>(self) -> ComponentCollection<C> {
    ComponentCollection {
      phantom: Default::default(),
      inner: self,
    }
  }
}

pub struct ComponentReadView<T: ComponentSemantic> {
  phantom: PhantomData<T>,
  pub(crate) inner: ComponentReadViewUntyped,
}

impl<T: ComponentSemantic> ComponentReadView<T> {
  pub unsafe fn get_by_untyped_handle(&self, idx: RawEntityHandle) -> Option<&T::Data> {
    self
      .inner
      .get(idx)
      .map(|v| unsafe { std::mem::transmute(v) })
  }

  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    unsafe { self.get_by_untyped_handle(idx.handle) }
  }
  pub fn get_without_generation_check(&self, idx: u32) -> Option<&T::Data> {
    self
      .inner
      .get_without_generation_check(idx.alloc_index())
      .map(|v| unsafe { std::mem::transmute(v) })
  }
  pub fn get_value(&self, idx: EntityHandle<T::Entity>) -> Option<T::Data> {
    self.get(idx).cloned()
  }
  pub fn get_value_without_generation_check(&self, idx: u32) -> Option<T::Data> {
    self.get_without_generation_check(idx).cloned()
  }
}

impl<T: ComponentSemantic> Clone for ComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      inner: self.inner.clone(),
    }
  }
}

pub struct ForeignKeyReadView<T: ForeignKeySemantic> {
  phantom: PhantomData<T>,
  data: ComponentReadView<T>,
}

impl<T: ForeignKeySemantic> ForeignKeyReadView<T> {
  pub fn get(
    &self,
    idx: EntityHandle<T::Entity>,
  ) -> Option<Option<EntityHandle<T::ForeignEntity>>> {
    self
      .data
      .get(idx)
      .map(|v| v.map(|v| unsafe { EntityHandle::<T::ForeignEntity>::from_raw(v) }))
  }
}

impl<T: ForeignKeySemantic> Clone for ForeignKeyReadView<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      data: self.data.clone(),
    }
  }
}

pub struct ComponentWriteView<T: ComponentSemantic> {
  phantom: PhantomData<T>,
  inner: ComponentWriteViewUntyped,
}

impl<T: ComponentSemantic> ComponentWriteView<T> {
  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    self
      .inner
      .get(idx.handle)
      .map(|v| unsafe { std::mem::transmute(v) })
  }

  pub fn read(&self, idx: EntityHandle<T::Entity>) -> Option<T::Data> {
    self.get(idx).cloned()
  }

  pub fn write(&mut self, idx: EntityHandle<T::Entity>, new: T::Data) -> bool {
    self.inner.write(idx.handle, &new as *const _ as DataPtr)
  }
}
