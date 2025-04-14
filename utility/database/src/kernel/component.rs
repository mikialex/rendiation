use crate::*;

#[derive(Clone)]
pub struct ComponentCollectionUntyped {
  /// the name of this component, only for display purpose
  pub name: String,

  /// mark if this component is a foreign key
  pub as_foreign_key: Option<EntityId>,

  /// the real data stored here
  pub data: Arc<dyn ComponentStorage>,

  /// watch this component all change with idx
  ///
  /// the type of `ChangePtr` is `ScopedMessage<DataType>`
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
      allocator: self.allocator.make_read_holder(),
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
    self.data.get(idx)
  }
}

pub struct ComponentWriteViewUntyped {
  data: Box<dyn ComponentStorageReadWriteView>,
  events: MutexGuardHolder<Source<ChangePtr>>,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl ComponentWriteViewUntyped {}

impl Drop for ComponentWriteViewUntyped {
  fn drop(&mut self) {
    self.data.notify_end_mutation(&mut self.events);
  }
}

impl ComponentWriteViewUntyped {
  pub fn get(&self, idx: RawEntityHandle) -> Option<DataPtr> {
    let _ = self.allocator.get(idx.0)?;
    self.data.get(idx.alloc_index())
  }

  pub fn write(&mut self, idx: RawEntityHandle, new: DataPtr) -> bool {
    self
      .data
      .set_value(idx, &new as *const _ as DataPtr, false, &mut self.events)
  }
  pub fn write_default(&mut self, idx: RawEntityHandle) -> bool {
    self.data.set_default_value(idx, false, &mut self.events)
  }

  pub fn delete(&mut self, idx: RawEntityHandle) {
    self.data.delete(idx, &mut self.events);
  }
}
