use crate::*;

pub struct EntityReaderUntyped {
  type_id: EntityId,
  components: smallvec::SmallVec<[(ComponentId, ComponentReadViewUntyped); 6]>,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl EntityReaderUntyped {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityReader<E>> {
    if self.type_id != E::entity_id() {
      return None;
    }
    EntityReader {
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
pub struct EntityReader<E: EntitySemantic> {
  phantom: PhantomData<E>, //
  inner: EntityReaderUntyped,
}

impl<E: EntitySemantic> EntityReader<E> {
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

impl<E: EntitySemantic> EntityComponentGroupTyped<E> {
  pub fn entity_reader(&self) -> EntityReader<E> {
    self.inner.entity_reader_dyn().into_typed().unwrap()
  }
}

impl EntityComponentGroup {
  pub fn entity_reader_dyn(&self) -> EntityReaderUntyped {
    let components = self.inner.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| (*id, c.read_untyped()))
      .collect();

    EntityReaderUntyped {
      type_id: self.inner.type_id,
      components,
      allocator: self.inner.allocator.make_read_holder(),
    }
  }
}
