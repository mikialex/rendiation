use crate::*;

pub struct ComponentCollection<'a, C> {
  phantom: PhantomData<C>,
  inner: &'a ComponentUntyped,
}

impl<'a, C: ComponentSemantic> ComponentCollection<'a, C> {
  pub fn read(&self) -> ComponentReadView<C> {
    ComponentReadView {
      phantom: PhantomData,
      inner: self.inner.read_untyped(),
    }
  }

  pub fn read_foreign_key(&self) -> ForeignKeyReadView<C>
  where
    C: ForeignKeySemantic,
  {
    ForeignKeyReadView {
      phantom: PhantomData,
      data: self.read(),
    }
  }

  pub fn write(&self) -> ComponentWriteView<C> {
    // see [Table::allocator] for the lock order
    let allocator = self.inner.allocator.make_read_holder();
    ComponentWriteView {
      phantom: PhantomData,
      inner: self.inner.write_untyped(),
      allocator,
    }
  }
}

impl ComponentUntyped {
  /// # Safety
  ///
  /// The C must match the real component semantic
  pub unsafe fn as_typed<C>(&self) -> ComponentCollection<'_, C> {
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
  /// # Safety
  ///
  /// The idx must match the real component semantic
  pub unsafe fn get_by_untyped_handle(&self, idx: RawEntityHandle) -> Option<&T::Data> {
    self
      .inner
      .get(idx)
      .map(|v| unsafe { &*(v as *const T::Data) })
  }

  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    unsafe { self.get_by_untyped_handle(idx.handle) }
  }

  pub fn get_without_generation_check(&self, idx: u32) -> Option<&T::Data> {
    self
      .inner
      .get_without_generation_check(idx.alloc_index())
      .map(|v| unsafe { &*(v as *const T::Data) })
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
  /// Get the foreign key of the entity, the returned option indicates if the foreign key is set.
  ///
  /// # Panics
  ///
  /// Panics if the entity handle is not alive, use [Self::try_get] if the handle may be invalid.
  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<EntityHandle<T::ForeignEntity>> {
    self
      .try_get(idx)
      .expect("the entity handle is not alive, use try_get if the handle may be invalid")
  }
  /// The outer option is none if the entity handle is not alive, the inner option indicates
  /// if the foreign key is set.
  pub fn try_get(
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
  allocator: LockReadGuardHolder<TableAllocator>,
}

impl<T: ComponentSemantic> ComponentWriteView<T> {
  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    self
      .inner
      .get(idx.handle, &self.allocator)
      .map(|v| unsafe { &*(v as *const T::Data) })
  }

  pub fn read(&self, idx: EntityHandle<T::Entity>) -> Option<T::Data> {
    self.get(idx).cloned()
  }

  /// return if write is valid
  pub fn write(&mut self, idx: EntityHandle<T::Entity>, new: T::Data) -> bool {
    let valid = self.allocator.get(idx.handle.0).is_some();
    unsafe {
      if valid {
        self.inner.write(idx.handle, &new as *const _ as DataPtr)
      }
    }
    valid
  }
}

#[test]
#[should_panic(expected = "the entity handle is not alive")]
fn foreign_key_get_with_dead_handle_should_panic() {
  declare_entity!(FkReadTestEntity);
  declare_foreign_key!(FkReadTestForeignKey, FkReadTestEntity, FkReadTestEntity);

  let db = Database::new(false);
  db.declare_entity::<FkReadTestEntity>()
    .declare_foreign_key::<FkReadTestForeignKey>();

  let entity = db.entity_writer::<FkReadTestEntity>().new_entity(|w| w);
  db.entity_writer::<FkReadTestEntity>().delete_entity(entity);

  let view = db.read_foreign_key::<FkReadTestForeignKey>();
  assert_eq!(view.try_get(entity), None);
  view.get(entity);
}
