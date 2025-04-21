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

    view.events.emit(&ScopedValueChange::Start);

    view
  }
}

pub type ChangeDataPtrPair<'a> = (DataPtr, &'a dyn DataBaseDataTypeDyn);

/// the lifetime of the ptr is only valid in the callback scope.
pub type ChangePtr = ScopedValueChange<(DataPtr, *const dyn DataBaseDataTypeDyn)>;

pub type EntityScopeChange = ScopedValueChange<()>;

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
    self.events.emit(&ScopedValueChange::End);
  }
}

impl ComponentWriteViewUntyped {
  pub fn get(&self, idx: RawEntityHandle, allocator: &Arena<()>) -> Option<DataPtr> {
    let _ = allocator.get(idx.0)?;
    unsafe { Some(self.data.get(idx.alloc_index())) }
  }

  /// # Safety
  ///
  /// idx must point to living data
  pub unsafe fn get_unchecked(&self, idx: RawEntityHandle) -> DataPtr {
    unsafe { self.data.get(idx.alloc_index()) }
  }

  /// # Safety
  ///
  /// idx must point to living data, data ptr must valid
  pub unsafe fn write(&mut self, idx: RawEntityHandle, init: bool, new: Option<DataPtr>) {
    let (new, old) = self.data.set_value(idx.index(), new);

    if init {
      let new_dyn = self.data.construct_dyn_datatype_from_raw_ptr(new);
      let new_dyn = new_dyn as *const dyn DataBaseDataTypeDyn;
      let pair = (new, new_dyn);
      let change = ValueChange::Delta(pair, None);
      let change = IndexValueChange { idx, change };
      self.events.emit(&ScopedValueChange::Message(change));
    } else {
      let new_dyn = self.data.construct_dyn_datatype_from_raw_ptr(new);
      let new_dyn = new_dyn as *const dyn DataBaseDataTypeDyn;
      let new_pair = (new, new_dyn);
      let old_dyn = self.data.construct_dyn_datatype_from_raw_ptr(old);
      let old_dyn = old_dyn as *const dyn DataBaseDataTypeDyn;
      let old_pair = (old, old_dyn);

      let change = ValueChange::Delta(new_pair, Some(old_pair));
      let change = IndexValueChange { idx, change };
      self.events.emit(&ScopedValueChange::Message(change));
    }
  }

  /// # Safety
  ///
  /// idx must point to living data
  pub unsafe fn delete(&mut self, idx: RawEntityHandle) {
    let old = self.data.delete(idx.index());

    let old_dyn = self.data.construct_dyn_datatype_from_raw_ptr(old);
    let old_dyn = old_dyn as *const dyn DataBaseDataTypeDyn;
    let old_pair = (old, old_dyn);

    let change = ValueChange::Remove(old_pair);
    let change = IndexValueChange { idx, change };
    self.events.emit(&ScopedValueChange::Message(change));
  }
}
