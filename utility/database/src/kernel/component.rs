use crate::*;

#[derive(Clone)]
pub struct ComponentCollectionUntyped {
  /// the name of this component, only for display purpose
  pub name: Arc<String>,

  /// mark if this component is a foreign key
  pub as_foreign_key: Option<EntityId>,

  /// the real data stored here
  pub data: Arc<dyn ComponentStorage>,

  /// watch this component all change with idx
  ///
  /// the type of [ChangePtr] is [ScopedValueChange<DataType>], the lifetime of the ptr is only valid
  /// in the callback scope.
  pub(crate) data_watchers: EventSource<ChangePtr>,

  pub allocator: Arc<RwLock<Arena<()>>>,

  pub data_typeid: TypeId,
  pub entity_type_id: EntityId,
  pub component_type_id: ComponentId,
}

impl ComponentCollectionUntyped {
  pub fn read_untyped(&self) -> ComponentReadViewUntyped {
    ComponentReadViewUntyped {
      allocator: self.allocator.make_read_holder(),
      data: self.data.create_read_view(),
    }
  }

  pub fn write_untyped(&self) -> ComponentWriteViewUntyped {
    let mut view = ComponentWriteViewUntyped {
      data: self.data.create_read_write_view(),
      events: self.data_watchers.lock.make_mutex_write_holder(),
    };

    view.data.notify_start_mutation(&mut view.events);

    view
  }
}

pub type ChangePtr = *const (); // ScopedValueChange<DataType>

#[derive(Clone)]
pub struct ComponentReadViewUntyped {
  pub(crate) allocator: LockReadGuardHolder<Arena<()>>,
  pub(crate) data: Box<dyn ComponentStorageReadView>,
}

impl ComponentReadViewUntyped {
  pub fn get(&self, idx: RawEntityHandle) -> Option<DataPtr> {
    let _ = self.allocator.get(idx.0)?;
    self.get_without_generation_check(idx.alloc_index())
  }
  pub fn get_without_generation_check(&self, idx: u32) -> Option<DataPtr> {
    // note, this is required, because the storage it self
    // do not has any information about if an idx has deleted before
    self.allocator.get_handle(idx as usize)?;
    self.data.get(idx)
  }
}

pub struct ComponentWriteViewUntyped {
  pub(crate) data: Box<dyn ComponentStorageReadWriteView>,
  events: MutexGuardHolder<Source<ChangePtr>>,
}

impl ComponentWriteViewUntyped {}

impl Drop for ComponentWriteViewUntyped {
  fn drop(&mut self) {
    self.data.notify_end_mutation(&mut self.events);
  }
}

impl ComponentWriteViewUntyped {
  pub fn get(&self, idx: RawEntityHandle, allocator: &Arena<()>) -> Option<DataPtr> {
    let _ = allocator.get(idx.0)?;
    self.data.get(idx.alloc_index())
  }

  pub fn write(&mut self, idx: RawEntityHandle, init: bool, new: DataPtr) -> bool {
    self.data.set_value(idx, new, init, &mut self.events)
  }
  pub fn write_default(&mut self, idx: RawEntityHandle, init: bool) -> bool {
    self.data.set_default_value(idx, init, &mut self.events)
  }

  pub fn delete(&mut self, idx: RawEntityHandle) {
    self.data.delete(idx, &mut self.events);
  }
}
