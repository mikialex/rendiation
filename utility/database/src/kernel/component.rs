use crate::*;

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

  pub arena: Arc<RwLock<Arena<()>>>,

  pub data_typeid: TypeId,
  pub entity_type_id: EntityId,
  pub component_type_id: ComponentId,
}

impl ComponentCollectionUntyped {
  pub fn read<C: ComponentSemantic>(&self) -> ComponentReadView<C> {
    ComponentReadView {
      phantom: PhantomData,
      data: self.data.create_read_view(),
      arena: self.arena.make_read_holder(),
    }
  }
  pub fn read_foreign_key<C: ForeignKeySemantic>(&self) -> ForeignKeyReadView<C> {
    ForeignKeyReadView {
      phantom: PhantomData,
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
      arena: self.arena.make_read_holder(),
      phantom: PhantomData,
    }
  }
}

pub type ChangePtr = *const (); // ScopedValueChange<DataType>

pub struct ComponentReadView<T: ComponentSemantic> {
  phantom: PhantomData<T>,
  arena: LockReadGuardHolder<Arena<()>>,
  data: Box<dyn ComponentStorageReadView>,
}

impl<T: ComponentSemantic> ComponentReadView<T> {
  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    todo!();
    // todo, generational check
    self.get_without_generation_check(idx.alloc_index())
  }
  pub fn get_without_generation_check(&self, idx: u32) -> Option<&T::Data> {
    self
      .data
      .get(idx)
      .map(|v| unsafe { std::mem::transmute(&*v) })
  }
  pub fn get_value(&self, idx: EntityHandle<T::Entity>) -> Option<T::Data> {
    self.get(idx).cloned()
  }
  pub fn get_value_without_generation_check(&self, idx: u32) -> Option<T::Data> {
    self.get_without_generation_check(idx).cloned()
  }
}

impl<T: ComponentSemantic> Clone for ComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      data: self.data.clone(),
      arena: self.arena.clone(),
    }
  }
}

pub struct ComponentWriteView<T: ComponentSemantic> {
  phantom: PhantomData<T>,
  pub(crate) data: Box<dyn ComponentStorageReadWriteView>,
  pub(crate) events: MutexGuardHolder<Source<ChangePtr>>,
  arena: LockReadGuardHolder<Arena<()>>,
}

impl<T: ComponentSemantic> Drop for ComponentWriteView<T> {
  fn drop(&mut self) {
    let message = ScopedMessage::<C::Data>::End;
    self.events.emit(&(&message as *const _ as ChangePtr));
  }
}

impl<T: ComponentSemantic> ComponentWriteView<T> {
  // pub fn with_write_value_persist(self, v: T::Data) -> impl EntityComponentWriter {
  //   EntityComponentWriterImpl {
  //     component: Some(self),
  //     default_value: move || v.clone(),
  //   }
  // }
  // pub fn with_write_value(self, v: T::Data) -> impl EntityComponentWriter {
  //   let mut v = Some(v);
  //   EntityComponentWriterImpl {
  //     component: Some(self),
  //     default_value: move || v.take().unwrap_or_default(),
  //   }
  // }
  // pub fn with_writer(self, f: impl FnMut() -> T::Data + 'static) -> impl EntityComponentWriter {
  //   EntityComponentWriterImpl {
  //     component: Some(self),
  //     default_value: f,
  //   }
  // }

  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<&T::Data> {
    self.data.get(idx.into())
  }

  pub fn read(&self, idx: EntityHandle<T::Entity>) -> Option<T::Data> {
    self.get(idx).cloned()
  }

  pub fn write(&mut self, idx: EntityHandle<T::Entity>, new: T::Data) {
    self.write_impl(idx, new, false);
  }

  pub(crate) fn write_impl(&mut self, idx: EntityHandle<T::Entity>, new: T::Data, is_create: bool) {
    let idx = idx.handle;
    let com = self.data.get_mut(idx).unwrap();
    let previous = std::mem::replace(com, new.clone());

    if is_create {
      let change = ValueChange::Delta(new, None);
      let change = IndexValueChange { idx, change };
      self.events.emit(&ScopedMessage::Message(change));
      return;
    }

    if previous == new {
      return;
    }

    let change = ValueChange::Delta(new, Some(previous));
    let change = IndexValueChange { idx, change };
    self.events.emit(&ScopedMessage::Message(change));
  }
}

pub struct ForeignKeyReadView<T: ForeignKeySemantic> {
  phantom: PhantomData<T>,
  data: Box<dyn ComponentStorageReadView>,
}

impl<T: ForeignKeySemantic> Clone for ForeignKeyReadView<T> {
  fn clone(&self) -> Self {
    Self {
      phantom: self.phantom,
      data: self.data.clone_read_view(),
    }
  }
}

impl<T: ForeignKeySemantic> ForeignKeyReadView<T> {
  pub fn get(&self, idx: EntityHandle<T::Entity>) -> Option<EntityHandle<T::ForeignEntity>> {
    self
      .data
      .get(idx.into())?
      .map(|v| unsafe { EntityHandle::<T::ForeignEntity>::from_raw(v) })
  }
}
