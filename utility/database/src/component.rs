use crate::*;

#[derive(Clone)]
pub struct ComponentCollection<T> {
  // todo make this optional static dispatch for better performance
  // todo remove arc
  pub(crate) data: Arc<dyn ComponentStorage<T>>,
  /// watch this component all change with idx
  pub(crate) group_watchers: EventSource<ComponentValueChange<T>>,
}

impl<T: CValue> ComponentCollection<T> {
  pub fn read(&self) -> ComponentReadView<T> {
    ComponentReadView {
      data: self.data.create_read_view(),
    }
  }
  pub fn write(&self) -> ComponentWriteView<T> {
    self.group_watchers.emit(&ComponentValueChange::StartWrite);
    ComponentWriteView {
      data: self.data.create_read_write_view(),
      events: self.group_watchers.lock.make_mutex_write_holder(),
    }
  }
}

impl<T: CValue + Default> Default for ComponentCollection<T> {
  fn default() -> Self {
    let data: Arc<RwLock<Vec<T>>> = Default::default();
    Self {
      data: Arc::new(data),
      group_watchers: Default::default(),
    }
  }
}

pub struct ComponentReadView<T: 'static> {
  data: Arc<dyn ComponentStorageReadView<T>>,
}

impl<T: 'static> Clone for ComponentReadView<T> {
  fn clone(&self) -> Self {
    Self {
      data: self.data.clone(),
    }
  }
}

pub struct IterableComponentReadView<T: 'static, E> {
  pub ecg: EntityComponentGroup<E>,
  pub read_view: Arc<dyn ComponentStorageReadView<T>>,
}

impl<T: 'static, E> Clone for IterableComponentReadView<T, E> {
  fn clone(&self) -> Self {
    Self {
      ecg: self.ecg.clone(),
      read_view: self.read_view.clone(),
    }
  }
}

// todo fix E constraint
impl<E: Any, T: CValue> VirtualCollection<AllocIdx<E>, T> for IterableComponentReadView<T, E> {
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (AllocIdx<E>, T)> + '_> {
    Box::new(
      self
        .ecg
        .iter_entity_idx()
        .map(|id| (id.into(), self.read_view.get(id as usize).cloned().unwrap())),
    )
  }

  fn access(&self, key: &AllocIdx<E>) -> Option<T> {
    self.read_view.get(key.index as usize).cloned()
  }
}

impl<T: CValue> ComponentReadView<T> {
  pub fn get(&self, idx: u32) -> Option<&T> {
    self.data.get(idx as usize)
  }
}

pub enum ComponentValueChange<T> {
  StartWrite,
  EndWrite,
  Write(IndexValueChange<T>),
}

pub struct IndexValueChange<T> {
  pub idx: u32,
  pub change: ValueChange<T>,
}

pub struct ComponentWriteView<T: 'static> {
  pub(crate) data: Box<dyn ComponentStorageReadWriteView<T>>,
  pub(crate) events: MutexGuardHolder<Source<ComponentValueChange<T>>>,
}

impl<T> Drop for ComponentWriteView<T> {
  fn drop(&mut self) {
    self.events.emit(&ComponentValueChange::EndWrite)
  }
}

impl<T: CValue + Default> ComponentWriteView<T> {
  pub fn with_writer(self, f: impl FnMut() -> T + 'static) -> impl EntityComponentWriter {
    EntityComponentWriterImpl {
      component: Some(self),
      default_value: f,
    }
  }

  pub fn write(&mut self, idx: u32, new: T) {
    self.write_impl(idx, new, false);
  }

  pub(crate) fn write_impl(&mut self, idx: u32, new: T, is_create: bool) {
    let com = self.data.get_mut(idx as usize).unwrap();
    let previous = std::mem::replace(com, new.clone());

    if is_create {
      let change = ValueChange::Delta(new, None);
      let change = IndexValueChange { idx, change };
      self.events.emit(&ComponentValueChange::Write(change));
      return;
    }

    if previous == new {
      return;
    }

    let change = ValueChange::Delta(new, Some(previous));
    let change = IndexValueChange { idx, change };
    self.events.emit(&ComponentValueChange::Write(change));
  }

  pub(crate) fn delete(&mut self, idx: u32) {
    let com = self.data.get_mut(idx as usize).unwrap();
    let previous = std::mem::take(com);

    let change = ValueChange::Remove(previous);
    let change = IndexValueChange { idx, change };
    self.events.emit(&ComponentValueChange::Write(change));
  }
}

pub trait DynamicComponent: Any + Send + Sync {
  fn create_dyn_writer_default(&self) -> Box<dyn EntityComponentWriter>;
  fn setup_new_storage(&mut self, storage: Box<dyn Any>);
  fn as_any(&self) -> &dyn Any;
}

impl<T: CValue + Default> DynamicComponent for ComponentCollection<T> {
  fn create_dyn_writer_default(&self) -> Box<dyn EntityComponentWriter> {
    Box::new(self.write().with_writer(T::default))
  }
  fn setup_new_storage(&mut self, storage: Box<dyn Any>) {
    self.data = *storage.downcast::<Arc<dyn ComponentStorage<T>>>().unwrap();
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
