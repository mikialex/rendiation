use crate::*;

pub struct TableReaderUntyped {
  type_id: EntityId,
  components: smallvec::SmallVec<[(ComponentId, ComponentReadViewUntyped); 6]>,
  allocator: LockReadGuardHolder<TableAllocator>,
}

impl TableReaderUntyped {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<TableReader<E>> {
    if self.type_id != E::entity_id() {
      return None;
    }
    TableReader {
      phantom: Default::default(),
      inner: self,
    }
    .into()
  }

  pub fn reconstruct_handle_by_idx(&self, idx: usize) -> Option<RawEntityHandle> {
    let handle = self.allocator.get_handle(idx);
    handle.map(RawEntityHandle)
  }
}

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct TableReader<E: EntitySemantic> {
  phantom: PhantomData<E>, //
  inner: TableReaderUntyped,
}

impl<E: EntitySemantic> TableReader<E> {
  pub fn reconstruct_handle_by_idx(&self, idx: usize) -> Option<EntityHandle<E>> {
    self
      .inner
      .reconstruct_handle_by_idx(idx)
      .map(|h| unsafe { EntityHandle::from_raw(h) })
  }

  pub fn get<C>(&self, idx: EntityHandle<C::Entity>) -> &C::Data
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_get::<C>(idx).unwrap()
  }

  pub fn try_get<C>(&self, idx: EntityHandle<C::Entity>) -> Option<&C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    for (id, view) in &self.inner.components {
      if *id == C::component_id() {
        unsafe {
          let data_ptr = view.get(idx.handle)?;
          let data_ptr: &C::Data = &*(data_ptr as *const C::Data);
          return Some(data_ptr);
        }
      }
    }
    None
  }

  pub fn read<C>(&self, idx: EntityHandle<C::Entity>) -> C::Data
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_read::<C>(idx).unwrap()
  }

  pub fn try_read<C>(&self, idx: EntityHandle<C::Entity>) -> Option<C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_get::<C>(idx).cloned()
  }

  pub fn read_ref<C>(&self, idx: EntityHandle<C::Entity>) -> &C::Data
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_read_ref::<C>(idx).unwrap()
  }

  pub fn try_read_ref<C>(&self, idx: EntityHandle<C::Entity>) -> Option<&C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_get::<C>(idx)
  }

  pub fn try_read_foreign_key<C>(
    &self,
    idx: EntityHandle<C::Entity>,
  ) -> Option<Option<EntityHandle<C::ForeignEntity>>>
  where
    C: ForeignKeySemantic<Entity = E>,
  {
    self
      .try_read::<C>(idx)
      .map(|v| v.map(|v| unsafe { EntityHandle::from_raw(v) }))
  }

  pub fn read_foreign_key<C>(
    &self,
    idx: EntityHandle<C::Entity>,
  ) -> Option<EntityHandle<C::ForeignEntity>>
  where
    C: ForeignKeySemantic<Entity = E>,
  {
    self.try_read_foreign_key::<C>(idx).unwrap()
  }

  pub fn read_expected_foreign_key<C>(
    &self,
    idx: EntityHandle<C::Entity>,
  ) -> EntityHandle<C::ForeignEntity>
  where
    C: ForeignKeySemantic<Entity = E>,
  {
    self.try_read_foreign_key::<C>(idx).unwrap().unwrap()
  }
}

impl<E: EntitySemantic> TypedArcTable<E> {
  pub fn entity_reader(&self) -> TableReader<E> {
    self.inner.entity_reader_dyn().into_typed().unwrap()
  }
}

impl ArcTable {
  pub fn entity_reader_dyn(&self) -> TableReaderUntyped {
    let components = self.internal.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| (*id, c.read_untyped()))
      .collect();

    TableReaderUntyped {
      type_id: self.internal.type_id,
      components,
      allocator: self.internal.allocator.make_read_holder(),
    }
  }
}
