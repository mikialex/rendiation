use crate::*;

/// This writer holds the all components write lock, and optimized for batch entity
/// creation and modification
pub struct EntityWriter<E: EntitySemantic> {
  phantom: PhantomData<E>, //
  inner: EntityWriterUntyped,
}

impl EntityWriterUntyped {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityWriter<E>> {
    if self.type_id != E::entity_id() {
      return None;
    }
    EntityWriter {
      phantom: Default::default(),
      inner: self,
    }
    .into()
  }
}

impl<E: EntitySemantic> EntityComponentGroupTyped<E> {
  pub fn entity_writer(&self) -> EntityWriter<E> {
    self.inner.entity_writer_dyn().into_typed().unwrap()
  }
}

impl<E: EntitySemantic> EntityWriter<E> {
  pub fn into_untyped(self) -> EntityWriterUntyped {
    self.inner
  }

  pub fn notify_reserve_changes(&mut self, count: usize) {
    self.inner.notify_reserve_changes(count);
  }

  pub fn mutate_component_data<C>(
    &mut self,
    idx: EntityHandle<C::Entity>,
    writer: impl FnOnce(&mut C::Data),
  ) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    if let Some(data) = self.try_read::<C>(idx) {
      let mut data = data.clone();
      writer(&mut data);
      self.write::<C>(idx, data);
    }
    self
  }

  pub fn write<C>(&mut self, idx: EntityHandle<C::Entity>, data: C::Data) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    let _ = self.inner.allocator.get(idx.handle.0).expect("bad handle");

    let view = self
      .inner
      .get_component_by_id_mut(C::component_id())
      .expect("unknown component");

    unsafe {
      // safety, we have checked handle validity above
      view.write_component(idx.handle, &data as *const C::Data as *const ());
    }

    self
  }

  pub fn write_foreign_key<C>(
    &mut self,
    idx: EntityHandle<C::Entity>,
    data: Option<EntityHandle<C::ForeignEntity>>,
  ) -> &mut Self
  where
    C: ForeignKeySemantic<Entity = E>,
  {
    self.write::<C>(idx, data.map(|d| d.handle))
  }

  pub fn try_read<C>(&self, idx: EntityHandle<C::Entity>) -> Option<C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    let view = self.inner.get_component_by_id(C::component_id()).unwrap();
    view
      .get(idx.handle, &self.inner.allocator)
      .map(|d| unsafe { &*(d as *const C::Data) })
      .cloned()
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
      .map(|idx| idx.map(|idx| unsafe { EntityHandle::from_raw(idx) }))
  }

  pub fn read<C>(&self, idx: EntityHandle<C::Entity>) -> C::Data
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.try_read::<C>(idx).unwrap()
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

  pub fn new_entity(
    &mut self,
    init: impl FnOnce(EntityInitWriteView) -> EntityInitWriteView,
  ) -> EntityHandle<E> {
    EntityHandle {
      handle: self.inner.new_entity(init),
      ty: PhantomData,
    }
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: EntityHandle<E>) {
    self.inner.delete_entity(handle.handle)
  }

  /// shallow clone
  pub fn clone_entity(&mut self, src: EntityHandle<E>) -> EntityHandle<E> {
    EntityHandle {
      handle: self.inner.clone_entity(src.handle),
      ty: PhantomData,
    }
  }
}
