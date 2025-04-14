use crate::*;

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct EntityWriter<E: EntitySemantic> {
  _phantom: SendSyncPhantomData<E>, //
  inner: EntityWriterUntyped,
}

impl<E: EntitySemantic> EntityComponentGroupTyped<E> {
  pub fn entity_writer(&self) -> EntityWriter<E> {
    self.inner.entity_writer_dyn().into_typed().unwrap()
  }
}

// https://users.rust-lang.org/t/why-closure-cannot-return-a-reference-to-data-moved-into-closure/72655/6
pub trait ComponentInitValueProvider {
  type Value;
  fn next_value(&mut self) -> Option<&Self::Value>;
}

struct ValueAsInitValueProviderPersist<T>(T);

impl<T> ComponentInitValueProvider for ValueAsInitValueProviderPersist<T> {
  type Value = T;
  fn next_value(&mut self) -> Option<&Self::Value> {
    Some(&self.0)
  }
}
struct ValueAsInitValueProviderOnce<T>(T, bool);

impl<T> ComponentInitValueProvider for ValueAsInitValueProviderOnce<T> {
  type Value = T;
  fn next_value(&mut self) -> Option<&Self::Value> {
    if self.1 {
      None
    } else {
      self.1 = true;
      Some(&self.0)
    }
  }
}

/// just the helper trait for [ComponentInitValueProvider]
pub trait UntypedComponentInitValueProvider {
  fn next_value(&mut self) -> Option<DataPtr>;
}

struct UntypedComponentInitValueProviderImpl<T>(T);

impl<T: ComponentInitValueProvider> UntypedComponentInitValueProvider
  for UntypedComponentInitValueProviderImpl<T>
{
  fn next_value(&mut self) -> Option<DataPtr> {
    let next = self.0.next_value();
    next.map(|next| next as *const T::Value as DataPtr)
  }
}

impl EntityComponentGroup {
  pub fn entity_writer_dyn(&self) -> EntityWriterUntyped {
    let components = self.inner.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| {
        (
          *id,
          EntityComponentWriterImpl {
            component: c.write_untyped(),
            next_value: None,
          },
        )
      })
      .collect();

    let change = ScopedValueChange::<()>::Start;
    self
      .inner
      .entity_watchers
      .emit(&(&change as *const _ as ChangePtr));

    EntityWriterUntyped {
      type_id: self.inner.type_id,
      components,
      entity_watchers: self.inner.entity_watchers.clone(),
      allocator: self.inner.allocator.make_write_holder(),
    }
  }
}

impl<E: EntitySemantic> EntityWriter<E> {
  pub fn into_untyped(self) -> EntityWriterUntyped {
    self.inner
  }

  pub fn with_component_value_writer<C>(mut self, value: C::Data) -> Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_value_writer::<C>(value);
    self
  }

  pub fn component_value_writer<C>(&mut self, value: C::Data) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_writer::<C>(ValueAsInitValueProviderOnce(value, false))
  }

  pub fn component_value_persist_writer<C>(&mut self, value: C::Data) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_writer::<C>(ValueAsInitValueProviderPersist(value))
  }

  pub fn with_component_writer<C>(
    mut self,
    writer_maker: impl ComponentInitValueProvider<Value = C::Data> + 'static,
  ) -> Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_writer::<C>(writer_maker);
    self
  }

  pub fn component_writer<C>(
    &mut self,
    writer_maker: impl ComponentInitValueProvider<Value = C::Data> + 'static,
  ) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    let view = self
      .inner
      .get_component_by_id_mut(C::component_id())
      .unwrap();
    let maker = UntypedComponentInitValueProviderImpl(writer_maker);
    view.next_value = Some(Box::new(maker));
    self
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
    let view = self
      .inner
      .get_component_by_id_mut(C::component_id())
      .unwrap();

    view.write_component(idx.handle, &data as *const C::Data as *const ());

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

  pub fn new_entity(&mut self) -> EntityHandle<E> {
    EntityHandle {
      handle: self.inner.new_entity(),
      ty: PhantomData,
    }
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: EntityHandle<E>) {
    self.inner.delete_entity(handle.handle)
  }
}

pub struct EntityWriterUntyped {
  type_id: EntityId,
  allocator: LockWriteGuardHolder<Arena<()>>,
  /// this change ptr type is ScopedValueChange<()>
  entity_watchers: EventSource<ChangePtr>,
  components: smallvec::SmallVec<[(ComponentId, EntityComponentWriterImpl); 6]>,
}

impl Drop for EntityWriterUntyped {
  fn drop(&mut self) {
    let change = ScopedValueChange::<()>::End;
    self
      .entity_watchers
      .emit(&(&change as *const _ as ChangePtr));
  }
}

impl EntityWriterUntyped {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityWriter<E>> {
    if self.type_id != E::entity_id() {
      return None;
    }
    EntityWriter {
      _phantom: Default::default(),
      inner: self,
    }
    .into()
  }

  pub fn get_component_by_id_mut(
    &mut self,
    id: ComponentId,
  ) -> Option<&mut EntityComponentWriterImpl> {
    self
      .components
      .iter_mut()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
  }
  pub fn get_component_by_id(&self, id: ComponentId) -> Option<&EntityComponentWriterImpl> {
    self
      .components
      .iter()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
  }

  pub fn new_entity(&mut self) -> RawEntityHandle {
    let handle = self.allocator.insert(());
    let handle = RawEntityHandle(handle);
    for com in &mut self.components {
      com.1.write_init_component_value(handle)
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Delta((), None),
    };

    let change = ScopedValueChange::Message(change);
    self
      .entity_watchers
      .emit(&(&change as *const _ as ChangePtr));
    handle
  }

  pub fn clone_entity(&mut self, src: RawEntityHandle) -> RawEntityHandle {
    let handle = self.allocator.insert(());
    let handle = RawEntityHandle(handle);
    assert!(self.allocator.contains(src.0));
    for com in &mut self.components {
      com.1.clone_component_value(src, handle, &self.allocator);
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Delta((), None),
    };
    let change = ScopedValueChange::Message(change);
    self
      .entity_watchers
      .emit(&(&change as *const _ as ChangePtr));
    handle
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: RawEntityHandle) {
    self.allocator.remove(handle.0).unwrap();
    for com in &mut self.components {
      com.1.delete_component(handle)
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Remove(()),
    };
    let change = ScopedValueChange::Message(change);
    self
      .entity_watchers
      .emit(&(&change as *const _ as ChangePtr));
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn uncheck_handle_delete_entity(&mut self, handle: u32) {
    let handle = self.allocator.get_handle(handle as usize).unwrap();
    self.delete_entity(RawEntityHandle(handle));
  }
}

pub struct EntityComponentWriterImpl {
  pub(crate) component: ComponentWriteViewUntyped,
  pub(crate) next_value: Option<Box<dyn UntypedComponentInitValueProvider>>,
}

impl EntityComponentWriterImpl {
  fn get(&self, idx: RawEntityHandle, allocator: &Arena<()>) -> Option<DataPtr> {
    self.component.get(idx, allocator)
  }

  fn write_component(&mut self, idx: RawEntityHandle, src: DataPtr) {
    self.component.write(idx, false, src);
  }

  fn write_init_component_value(&mut self, idx: RawEntityHandle) {
    self.component.data.grow(idx.index());
    if let Some(next_value_maker) = self.next_value.as_mut() {
      if let Some(next) = next_value_maker.next_value() {
        self.component.write(idx, true, next);
      } else {
        self.component.write_default(idx, true);
      }
    } else {
      self.component.write_default(idx, true);
    }
  }
  fn clone_component_value(
    &mut self,
    src: RawEntityHandle,
    dst: RawEntityHandle,
    allocator: &Arena<()>,
  ) {
    let src = self.get(src, allocator).unwrap();
    self.write_component(dst, src);
  }

  fn delete_component(&mut self, idx: RawEntityHandle) {
    self.component.delete(idx);
  }
}
