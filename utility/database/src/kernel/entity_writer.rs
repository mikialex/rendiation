use crate::*;

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

pub struct EntityWriterUntyped {
  pub(crate) type_id: EntityId,
  pub(crate) allocator: LockWriteGuardHolder<Arena<()>>,
  /// this change ptr type is ScopedValueChange<()>, the lifetime of the ptr is only valid
  /// in the callback scope.
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
  pub fn get(&self, idx: RawEntityHandle, allocator: &Arena<()>) -> Option<DataPtr> {
    self.component.get(idx, allocator)
  }

  pub fn write_component(&mut self, idx: RawEntityHandle, src: DataPtr) {
    self.component.write(idx, false, src);
  }

  pub fn write_init_component_value(&mut self, idx: RawEntityHandle) {
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
  pub fn clone_component_value(
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
