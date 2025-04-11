use crate::*;

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct EntityReader<E: EntitySemantic> {
  _phantom: SendSyncPhantomData<E>, //
  inner: EntityReaderUntyped,
}

impl<E: EntitySemantic> EntityReader<E> {
  pub fn reconstruct_handle_by_idx(&self, idx: usize) -> Option<EntityHandle<E>> {
    let handle = self.inner._allocator.get_handle(idx);
    handle.map(|h| unsafe { EntityHandle::from_raw(RawEntityHandle(h)) })
  }

  pub fn read<C>(&self, idx: EntityHandle<C::Entity>) -> C::Data
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_read::<C>(idx).unwrap()
  }

  pub fn try_read<C>(&self, idx: EntityHandle<C::Entity>) -> Option<&C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    if self.inner._allocator.get(idx.handle.0).is_none() {
      return None;
    }
    for (id, view) in &self.inner.components {
      if *id == C::component_id() {
        unsafe {
          let data_ptr = view.get_unchecked(idx.index);
          let data_ptr: &C::Data = unsafe { std::mem::transmute(data_ptr) };
          return Some(data_ptr);
        }
        break;
      }
    }
    None
  }

  pub fn try_read_foreign_key<C>(
    &self,
    idx: EntityHandle<C::Entity>,
  ) -> Option<Option<EntityHandle<C::ForeignEntity>>>
  where
    C: ForeignKeySemantic<Entity = E>,
  {
    self
      .try_read(idx)
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
      .map(|(id, c)| (*id, c.inner.create_dyn_reader()))
      .collect();

    EntityReaderUntyped {
      type_id: self.inner.type_id,
      components,
      _allocator: self.inner.allocator.make_read_holder(),
    }
  }
}

pub struct EntityReaderUntyped {
  type_id: EntityId,
  /// just to hold this lock to prevent any entity creation or deletion
  _allocator: LockReadGuardHolder<Arena<()>>,
  components: smallvec::SmallVec<[(ComponentId, Box<dyn ComponentStorageReadView>); 6]>,
}

impl EntityReaderUntyped {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityReader<E>> {
    if self.type_id != E::entity_id() {
      return None;
    }
    EntityReader {
      _phantom: Default::default(),
      inner: self,
    }
    .into()
  }
}
