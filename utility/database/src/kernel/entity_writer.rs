use crate::*;

impl ArcTable {
  pub fn entity_writer_dyn(&self) -> EntityWriterUntyped {
    let change: ChangePtr = ScopedValueChange::Start;
    self.internal.entity_watchers.emit(&change);

    let components = self.internal.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| {
        (
          *id,
          EntityComponentWriterImpl {
            component: c.write_untyped(),
            has_write_for_new_entity: false,
          },
        )
      })
      .collect();

    EntityWriterUntyped {
      type_id: self.internal.type_id,
      components,
      entity_watchers: self.internal.entity_watchers.clone(),
      allocator: self.internal.allocator.make_write_holder(),
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

pub struct EntityInitWriteView<'a> {
  components: &'a mut [(ComponentId, EntityComponentWriterImpl)],
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

impl Drop for EntityWriterUntyped {
  fn drop(&mut self) {
    let change: ChangePtr = ScopedValueChange::End;
    self.components.clear(); // trigger the components writer's dropper first
    self.entity_watchers.emit(&change);
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

  pub fn notify_reserve_changes(&mut self, count: usize) {
    self
      .entity_watchers
      .emit(&ScopedValueChange::ReserveSpace(count));

    for (_, com) in &mut self.components {
      com.component.notify_reserve_changes(count);
    }
  }

  pub fn new_entity(
    &mut self,
    writer: impl FnOnce(EntityInitWriteView) -> EntityInitWriteView,
  ) -> RawEntityHandle {
    let handle = self.allocator.insert(());
    let handle = RawEntityHandle(handle);

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

    let new_pair = (&() as DataPtr, &() as &dyn DataBaseDataTypeDyn as *const _);
    let change = ValueChange::Delta(new_pair, None);
    let change = IndexValueChange {
      idx: handle,
      change,
    };
    let change = ScopedValueChange::Message(change);
    self.entity_watchers.emit(&change);
    handle
  }

  pub fn clone_entity(&mut self, src: RawEntityHandle) -> RawEntityHandle {
    let handle = self.allocator.insert(());
    let handle = RawEntityHandle(handle);
    assert!(self.allocator.contains(src.0));
    for com in &mut self.components {
      unsafe {
        // safety, the handle is just created.
        com.1.clone_component_value(src, handle);
      }
    }
    let new_pair = (&() as DataPtr, &() as &dyn DataBaseDataTypeDyn as *const _);
    let change = ValueChange::Delta(new_pair, None);
    let change = IndexValueChange {
      idx: handle,
      change,
    };
    let change = ScopedValueChange::Message(change);
    self.entity_watchers.emit(&change);
    handle
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: RawEntityHandle) {
    self.allocator.remove(handle.0).unwrap();
    for com in &mut self.components {
      unsafe {
        // safety, the handle is just got removed, so it's valid for components.
        com.1.delete_component(handle)
      }
    }

    let pair = (&() as DataPtr, &() as &dyn DataBaseDataTypeDyn as *const _);
    let change = ValueChange::Remove(pair);
    let change = IndexValueChange {
      idx: handle,
      change,
    };
    let change = ScopedValueChange::Message(change);
    self.entity_watchers.emit(&change);
  }
}

pub struct EntityComponentWriterImpl {
  pub(crate) component: ComponentWriteViewUntyped,
  pub(crate) has_write_for_new_entity: bool,
}

impl EntityComponentWriterImpl {
  pub fn get(&self, idx: RawEntityHandle, allocator: &Arena<()>) -> Option<DataPtr> {
    self.component.get(idx, allocator)
  }

  /// # Safety
  ///
  /// idx must point to living data
  pub unsafe fn write_component(&mut self, idx: RawEntityHandle, src: DataPtr) {
    self.component.write(idx, false, Some(src));
  }

  /// # Safety
  ///
  /// idx must allocated
  pub unsafe fn write_init_component_value(&mut self, idx: RawEntityHandle, data: Option<DataPtr>) {
    self.component.data.deref_mut().resize(idx.index());
    self.component.write(idx, true, data);
  }

  /// # Safety
  ///
  /// idx must point to living data
  pub unsafe fn clone_component_value(&mut self, src: RawEntityHandle, dst: RawEntityHandle) {
    let src = self.component.get_unchecked(src);
    self.component.data.deref_mut().resize(dst.index());
    self.write_component(dst, src);
  }

  unsafe fn delete_component(&mut self, idx: RawEntityHandle) {
    self.component.delete(idx);
  }
}
