use crate::*;

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct EntityReader<E: EntitySemantic> {
  _phantom: SendSyncPhantomData<E>, //
  inner: EntityReaderUntyped,
}

impl<E: EntitySemantic> EntityReader<E> {
  pub fn read_component_data<C>(&self, idx: EntityHandle<C::Entity>) -> Option<C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    let mut target = None;
    for (id, view) in &self.inner.components {
      if *id == C::component_id() {
        unsafe {
          let target = &mut target as *mut Option<C::Data> as *mut ();
          view.read_component(idx.handle, target);
        }
      }
    }
    target
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

    self.inner.entity_watchers.emit(&ScopedMessage::Start);

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
  components: smallvec::SmallVec<[(ComponentId, Box<dyn EntityComponentReader>); 6]>,
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

pub trait EntityComponentReader {
  /// # Safety
  /// target's type is Option<T>, if read success, the implementation should cast the target and
  /// set the read value.
  unsafe fn read_component(&self, idx: RawEntityHandle, target: *mut ());
}
