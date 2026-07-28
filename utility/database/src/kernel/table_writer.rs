use crate::*;

impl ArcTable {
  pub fn entity_writer_dyn(&self) -> TableWriterUntyped {
    let change = ScopedMessage::Start;
    self.internal.entity_watchers.emit(&change);

    let components = self.internal.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| {
        (
          *id,
          TableWriterImpl {
            component: c.write_untyped(),
            has_write_for_new_entity: false,
          },
        )
      })
      .collect();

    TableWriterUntyped {
      type_id: self.internal.type_id,
      components,
      entity_watchers: self.internal.entity_watchers.clone(),
      allocator: self.internal.allocator.make_write_holder(),
    }
  }
}

pub struct TableWriterUntyped {
  pub(crate) type_id: EntityId,
  pub(crate) allocator: LockWriteGuardHolder<TableAllocator>,
  /// this change ptr type is ScopedValueChange<()>, the lifetime of the ptr is only valid
  /// in the callback scope.
  entity_watchers: EventSource<EntityChangeMessage>,
  components: smallvec::SmallVec<[(ComponentId, TableWriterImpl); 6]>,
}

pub struct EntityInitWriteView<'a> {
  components: &'a mut [(ComponentId, TableWriterImpl)],
  idx: RawEntityHandle,
}

impl<'a> EntityInitWriteView<'a> {
  pub fn write<C: ComponentSemantic>(self, v: &C::Data) -> Self {
    let id = C::component_id();
    let com = self
      .components
      .iter_mut()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
      .expect("unknown component");

    unsafe {
      com.write_init_component_value(self.idx, Some(v as *const C::Data as DataPtr));
      com.has_write_for_new_entity = true;
    }

    self
  }
}

impl Drop for TableWriterUntyped {
  fn drop(&mut self) {
    let change = ScopedMessage::End;
    self.components.clear(); // trigger the components writer's dropper first
    self.entity_watchers.emit(&change);
  }
}

impl TableWriterUntyped {
  pub fn get_component_by_id_mut(&mut self, id: ComponentId) -> Option<&mut TableWriterImpl> {
    self
      .components
      .iter_mut()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
  }
  pub fn get_component_by_id(&self, id: ComponentId) -> Option<&TableWriterImpl> {
    self
      .components
      .iter()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
  }

  pub fn notify_reserve_changes(&mut self, count: usize) {
    self
      .entity_watchers
      .emit(&ScopedMessage::ReserveSpace(count));

    for (_, com) in &mut self.components {
      com.component.notify_reserve_changes(count);
    }
  }

  pub fn new_entity(
    &mut self,
    writer: impl FnOnce(EntityInitWriteView) -> EntityInitWriteView,
  ) -> RawEntityHandle {
    let cap_before = self.allocator.capacity();
    let handle = self.allocator.insert(());
    let cap_after = self.allocator.capacity();

    let handle = RawEntityHandle(handle);

    let change = EntityChange::NewEntityStartCreate(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);

    if cap_after > cap_before {
      for com in &mut self.components {
        unsafe { com.1.resize(cap_after as u32) };
      }
    }

    writer(EntityInitWriteView {
      components: &mut self.components,
      idx: handle,
    });

    for com in &mut self.components {
      if !com.1.has_write_for_new_entity {
        unsafe {
          // safety, the handle has just created.
          com.1.write_init_component_value(handle, None);
        }
      } else {
        com.1.has_write_for_new_entity = false
      }
    }

    let change = EntityChange::NewEntityCreated(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);
    handle
  }

  pub fn clone_entity(&mut self, src: RawEntityHandle) -> RawEntityHandle {
    assert!(self.allocator.contains(src.0));

    let cap_before = self.allocator.capacity();
    let handle = self.allocator.insert(());
    let cap_after = self.allocator.capacity();

    let handle = RawEntityHandle(handle);

    let change = EntityChange::NewEntityStartCreate(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);

    for com in &mut self.components {
      unsafe {
        if cap_after > cap_before {
          com.1.resize(cap_after as u32);
        }
        // safety, the handle is just created.
        com.1.clone_component_value(src, handle);
      }
    }

    let change = EntityChange::NewEntityCreated(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);

    handle
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: RawEntityHandle) {
    let cap_before = self.allocator.capacity();
    self.allocator.remove(handle.0).unwrap();
    let cap_after = self.allocator.capacity();
    for com in &mut self.components {
      unsafe {
        // safety, the handle is just got removed, so it's valid for components.
        com.1.delete_component(handle);
        if cap_after < cap_before {
          com.1.resize(cap_after as u32);
        }
      }
    }

    let change = EntityChange::DeleteEntity(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);
  }
}

pub struct TableWriterImpl {
  pub(crate) component: ComponentWriteViewUntyped,
  pub(crate) has_write_for_new_entity: bool,
}

impl TableWriterImpl {
  pub fn get(&self, idx: RawEntityHandle, allocator: &TableAllocator) -> Option<DataPtr> {
    self.component.get(idx, allocator)
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value]
  pub unsafe fn write_component(&mut self, idx: RawEntityHandle, src: DataPtr) {
    self.component.write(idx, src);
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value_init]
  pub unsafe fn write_init_component_value(&mut self, idx: RawEntityHandle, data: Option<DataPtr>) {
    self.component.init(idx, data);
  }

  /// # Safety
  ///
  /// idx must point to living data
  pub unsafe fn clone_component_value(&mut self, src: RawEntityHandle, dst: RawEntityHandle) {
    let src = self.component.get_unchecked(src);
    self.write_component(dst, src);
  }

  unsafe fn delete_component(&mut self, idx: RawEntityHandle) {
    self.component.delete(idx);
  }

  /// # Safety
  ///
  /// see [ComponentStorageReadViewBase::resize]
  unsafe fn resize(&mut self, max_cap: u32) {
    self.component.data.deref_mut().resize(max_cap);
  }
}
