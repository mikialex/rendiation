use crate::*;

pub struct ComponentUntyped {
  /// the name of this component, must be unique among all components
  pub name: String,

  /// the name for ui display
  pub short_name: String,

  /// indicate if this component is a foreign key
  pub as_foreign_key: Option<EntityId>,

  /// the real data stored here
  pub data: Box<dyn ComponentStorage>,

  /// watch this component all change with idx
  pub data_watchers: EventSource<ChangePtr>,

  /// the allocator that shared between all components of the self entity
  pub allocator: Arc<RwLock<TableAllocator>>,

  pub data_meta: DataTypeMetaInfo,
  pub entity_type_id: EntityId,
  pub component_type_id: ComponentId,

  /// Converts serialized component data to a human-readable debug string.
  /// Returns `None` if deserialization fails.
  pub binary_to_debug_string: fn(DatabaseSerializedFieldBuffer) -> Option<String>,
}

impl ComponentUntyped {
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

pub type ChangeDataPtrPair<'a> = (DataPtr, &'a dyn DynDataBaseDataType);

/// the lifetime of the ptr is only valid in the callback scope.
pub type ChangePtr = ScopedValueChange<(DataPtr, *const dyn DynDataBaseDataType)>;

pub type EntityChangeMessage = ScopedMessage<EntityChange>;

pub enum EntityChange {
  NewEntityStartCreate(RawEntityHandle),
  NewEntityCreated(RawEntityHandle),
  DeleteEntity(RawEntityHandle),
}

pub struct ComponentReadViewUntyped {
  pub(crate) allocator: LockReadGuardHolder<TableAllocator>,
  pub(crate) data: ComponentReadViewBox,
}

impl Clone for ComponentReadViewUntyped {
  fn clone(&self) -> Self {
    Self {
      allocator: self.allocator.clone(),
      data: self.data.clone_boxed(),
    }
  }
}

impl ComponentReadViewUntyped {
  #[inline]
  pub fn get(&self, idx: RawEntityHandle) -> Option<DataPtr> {
    // note, this is required, because the storage itself
    // do not have any information about if an idx has deleted before
    self.allocator.get(idx.0)?;
    self.get_without_generation_check(idx.alloc_index())
  }

  #[inline]
  pub fn get_without_generation_check(&self, idx: u32) -> Option<DataPtr> {
    self.allocator.get_handle(idx as usize)?;
    unsafe { Some(self.data.get(idx)) }
  }

  #[inline]
  pub fn get_without_generation_check_dyn_data_type(
    &self,
    idx: u32,
  ) -> Option<(DataPtr, &dyn DynDataBaseDataType)> {
    self.allocator.get_handle(idx as usize)?;
    unsafe { Some((self.data.get(idx), self.data.get_as_dyn_storage(idx))) }
  }
}

pub struct ComponentWriteViewUntyped {
  pub(crate) data: ComponentReadWriteViewBox,
  events: MutexGuardHolder<Source<ChangePtr>>,
}

impl ComponentWriteViewUntyped {}

impl Drop for ComponentWriteViewUntyped {
  fn drop(&mut self) {
    self.events.emit(&ScopedValueChange::End);
  }
}

impl ComponentWriteViewUntyped {
  pub fn get(&self, idx: RawEntityHandle, allocator: &TableAllocator) -> Option<DataPtr> {
    let _ = allocator.get(idx.0)?;
    unsafe { Some(self.data.get(idx.alloc_index())) }
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadViewBase::get]
  pub unsafe fn get_unchecked(&self, idx: RawEntityHandle) -> DataPtr {
    unsafe { self.data.get(idx.alloc_index()) }
  }

  pub fn notify_reserve_changes(&mut self, count: usize) {
    self.events.emit(&ScopedValueChange::ReserveSpace(count));
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value_init]
  pub unsafe fn init(&mut self, idx: RawEntityHandle, new: Option<DataPtr>) {
    let new = self.data.set_value_init(idx.index(), new);

    let new_dyn = self.data.meta().construct_dyn_datatype_from_raw_ptr(new);
    let new_dyn = new_dyn as *const dyn DynDataBaseDataType;
    let pair = (new, new_dyn);
    let change = ValueChange::Delta(pair, None);
    let change = IndexValueChange { idx, change };
    self.events.emit(&ScopedValueChange::Message(change));
  }

  unsafe fn emit_delta(&mut self, idx: RawEntityHandle, new: DataPtr, old: DataPtr) {
    let new_dyn = self.data.meta().construct_dyn_datatype_from_raw_ptr(new);
    let new_dyn = new_dyn as *const dyn DynDataBaseDataType;
    let new_pair = (new, new_dyn);

    let old_dyn = self.data.meta().construct_dyn_datatype_from_raw_ptr(old);
    let old_dyn = old_dyn as *const dyn DynDataBaseDataType;
    let old_pair = (old, old_dyn);

    let change = ValueChange::Delta(new_pair, Some(old_pair));
    let change = IndexValueChange { idx, change };
    self.events.emit(&ScopedValueChange::Message(change));
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value]
  pub unsafe fn write(&mut self, idx: RawEntityHandle, new: DataPtr) {
    let (new, old, changed) = self.data.set_value(idx.index(), new);

    if changed {
      self.emit_delta(idx, new, old);
    }
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value]
  pub unsafe fn write_by_small_serialize_data(
    &mut self,
    idx: RawEntityHandle,
    new: DatabaseSerializedFieldBufferOrForeignKey,
  ) {
    let (new, old, changed) = self
      .data
      .set_value_from_serialize_field_data(idx.index(), new);

    if changed {
      self.emit_delta(idx, new, old);
    }
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::delete]
  pub unsafe fn delete(&mut self, idx: RawEntityHandle) {
    let old = self.data.delete(idx.index());

    let old_dyn = self.data.meta().construct_dyn_datatype_from_raw_ptr(old);
    let old_dyn = old_dyn as *const dyn DynDataBaseDataType;
    let old_pair = (old, old_dyn);

    let change = ValueChange::Remove(old_pair);
    let change = IndexValueChange { idx, change };
    self.events.emit(&ScopedValueChange::Message(change));
  }
}
