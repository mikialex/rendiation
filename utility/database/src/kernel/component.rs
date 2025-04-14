use crate::*;

#[derive(Clone)]
pub struct ComponentCollectionUntyped {
  /// the name of this component, only for display purpose
  pub name: String,

  /// mark if this component is a foreign key
  pub as_foreign_key: Option<EntityId>,

  /// the real data stored here
  pub(crate) data: Arc<dyn ComponentStorage>,

  /// watch this component all change with idx
  ///
  /// the type of `ChangePtr` is `ScopedMessage<DataType>`
  pub(crate) group_watchers: EventSource<ChangePtr>,

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

  pub fn write<C: ComponentSemantic>(&self) -> ComponentWriteView<C> {
    let message = ScopedMessage::<C::Data>::Start;
    self
      .group_watchers
      .emit(&(&message as *const _ as ChangePtr));
    ComponentWriteView {
      data: self.data.create_read_write_view(),
      events: self.group_watchers.lock.make_mutex_write_holder(),
      allocator: self.allocator.make_read_holder(),
      phantom: PhantomData,
    }
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

pub struct ComponentWriteView<T: ComponentSemantic> {
  phantom: PhantomData<T>,
  pub(crate) data: Box<dyn ComponentStorageReadWriteView>,
  pub(crate) events: MutexGuardHolder<Source<ChangePtr>>,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl<T: ComponentSemantic> Drop for ComponentWriteView<T> {
  fn drop(&mut self) {
    let message = ScopedMessage::<T::Data>::End;
    self.events.emit(&(&message as *const _ as ChangePtr));
  }
}

impl<T: ComponentSemantic> ComponentWriteView<T> {
  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    let _ = self.allocator.get(idx.handle.0)?;
    self
      .data
      .get(idx.alloc_index())
      .map(|v| unsafe { std::mem::transmute(v) })
  }

  pub fn read(&self, idx: EntityHandle<T::Entity>) -> Option<T::Data> {
    self.get(idx).cloned()
  }

  pub fn write(&mut self, idx: EntityHandle<T::Entity>, new: T::Data) {
    self.write_impl(idx, new, false);
  }

  pub(crate) fn write_impl(
    &mut self,
    index: EntityHandle<T::Entity>,
    new: T::Data,
    is_create: bool,
  ) {
    let idx = index.alloc_index();
    let com = self
      .data
      .get_mut(idx)
      .map(|v| unsafe { std::mem::transmute(v) })
      .unwrap();
    let previous = std::mem::replace(com, new.clone());

    let idx = index.handle;

    if is_create {
      let change = ValueChange::Delta(new, None);
      let change = IndexValueChange { idx, change };
      let msg = ScopedMessage::Message(change);
      self.events.emit(&(&msg as *const _ as ChangePtr));
      return;
    }

    if previous == new {
      return;
    }

    let change = ValueChange::Delta(new, Some(previous));
    let change = IndexValueChange { idx, change };
    let msg = ScopedMessage::Message(change);
    self.events.emit(&(&msg as *const _ as ChangePtr));
  }
}
