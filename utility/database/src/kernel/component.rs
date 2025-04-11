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

  pub data_typeid: TypeId,
  pub entity_type_id: EntityId,
  pub component_type_id: ComponentId,
}

impl ComponentCollectionUntyped {
  //   fn new() -> Self {
  //     let data: Arc<RwLock<Vec<C::Data>>> = Default::default();
  //     Self {
  //       phantom: PhantomData,
  //       data: Arc::new(data),
  //       group_watchers: Default::default(),
  //     }
  //   }

  pub fn read<C: ComponentSemantic>(&self) -> ComponentReadView<C> {
    ComponentReadView {
      phantom: PhantomData,
      data: self.data.create_read_view(),
    }
  }
  pub fn read_foreign_key<C: ForeignKeySemantic>(&self) -> ForeignKeyReadView<C>
  where
    C: ForeignKeySemantic,
  {
    ForeignKeyReadView {
      phantom: PhantomData,
      data: self.data.create_read_view(),
    }
  }

  pub fn write<C: ComponentSemantic>(&self) -> ComponentWriteView<C> {
    let message = ScopedMessage::Start;
    self
      .group_watchers
      .emit(&(&message as *const _ as ChangePtr));
    ComponentWriteView {
      data: self.data.create_read_write_view(),
      events: self.group_watchers.lock.make_mutex_write_holder(),
    }
  }
}

pub type ChangePtr = *const (); // ScopedValueChange<DataType>

pub struct ComponentReadView<T: ComponentSemantic> {
  phantom: PhantomData<T>,
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
    }
  }
}

pub struct IterableComponentReadView<T: ComponentSemantic> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentCollection<T>,
}

impl<T: ComponentSemantic> Clone for IterableComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
    }
  }
}

impl<T: ComponentSemantic<Data: CValue>> Query for IterableComponentReadView<T> {
  type Key = u32;
  type Value = T::Data;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, T::Data)> + '_ {
    self
      .ecg
      .iter_entity_idx()
      .map(|id| (id.alloc_index(), self.read_view.get(id).cloned().unwrap()))
  }

  fn access(&self, key: &u32) -> Option<T::Data> {
    self.read_view.get_without_generation_check(*key).cloned()
  }
}

pub struct IterableComponentReadViewChecked<T: ComponentSemantic> {
  pub ecg: EntityComponentGroup,
  pub read_view: ComponentReadView<T>,
}

impl<T: ComponentSemantic> Clone for IterableComponentReadViewChecked<T> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
    }
  }
}

impl<T: ComponentSemantic<Data: CValue>> Query for IterableComponentReadViewChecked<T> {
  type Key = RawEntityHandle;
  type Value = T::Data;
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, T::Data)> + '_ {
    self.ecg.iter_entity_idx().map(|id| {
      (
        id,
        self
          .read_view
          .get_value_without_generation_check(id.index())
          .unwrap(),
      )
    })
  }

  fn access(&self, key: &RawEntityHandle) -> Option<T::Data> {
    self.read_view.get_value(*key)
  }
}

pub struct ComponentWriteView<T: ComponentSemantic> {
  pub(crate) data: Box<dyn ComponentStorageReadWriteView<T::Data>>,
  pub(crate) events: MutexGuardHolder<Source<ChangePtr>>,
}

impl<T: ComponentSemantic> Drop for ComponentWriteView<T> {
  fn drop(&mut self) {
    self.events.emit(&ScopedValueChange::End)
  }
}

impl<T: ComponentSemantic> ComponentWriteView<T> {
  pub fn with_write_value_persist(self, v: T::Data) -> impl EntityComponentWriter {
    EntityComponentWriterImpl {
      component: Some(self),
      default_value: move || v.clone(),
    }
  }
  pub fn with_write_value(self, v: T::Data) -> impl EntityComponentWriter {
    let mut v = Some(v);
    EntityComponentWriterImpl {
      component: Some(self),
      default_value: move || v.take().unwrap_or_default(),
    }
  }
  pub fn with_writer(self, f: impl FnMut() -> T::Data + 'static) -> impl EntityComponentWriter {
    EntityComponentWriterImpl {
      component: Some(self),
      default_value: f,
    }
  }

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

  pub(crate) fn delete(&mut self, idx: EntityHandle<T::Entity>) {
    let idx = idx.handle;
    let com = self.data.get_mut(idx).unwrap();
    let previous = std::mem::take(com);

    let change = ValueChange::Remove(previous);
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

// pub trait UntypedComponentCollectionLike: Any + Send + Sync {
//   /// return the debug display for the stored value.
//   fn debug_value(&self, idx: usize) -> Option<String>;

//   fn create_dyn_writer_default(&self) -> Box<dyn EntityComponentWriter>;
//   fn create_dyn_reader(&self) -> Box<dyn EntityComponentReader>;

//   // todo, this breaks previous get_data returned storage
//   fn setup_new_storage(&mut self, storage: Box<dyn Any>);
//   fn get_data(&self) -> Box<dyn Any>;
//   /// could be downcast to EventSource<ScopedValueChange<T>>
//   fn get_event_source(&self) -> Box<dyn Any>;
//   /// could be downcast to ComponentCollection<T>
//   fn as_any(&self) -> &dyn Any;
// }

// impl<T: ComponentSemantic> UntypedComponentCollectionLike for ComponentCollection<T> {
//   fn debug_value(&self, idx: usize) -> Option<String> {
//     self
//       .read()
//       .get_without_generation_check(idx as u32)
//       .map(|v| format!("{:?}", v))
//   }
//   fn create_dyn_writer_default(&self) -> Box<dyn EntityComponentWriter> {
//     Box::new(self.write().with_writer(T::default_override))
//   }
//   fn create_dyn_reader(&self) -> Box<dyn EntityComponentReader> {
//     Box::new(self.read())
//   }
//   fn setup_new_storage(&mut self, storage: Box<dyn Any>) {
//     self.data = *storage
//       .downcast::<Arc<dyn ComponentStorage<T::Data>>>()
//       .unwrap();
//   }
//   fn get_data(&self) -> Box<dyn Any> {
//     Box::new(self.data.clone())
//   }
//   fn get_event_source(&self) -> Box<dyn Any> {
//     Box::new(self.group_watchers.clone())
//   }
//   fn as_any(&self) -> &dyn Any {
//     self
//   }
// }
