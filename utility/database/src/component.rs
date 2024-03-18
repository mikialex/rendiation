use crate::*;

#[derive(Clone)]
pub struct ComponentCollection<T> {
  // todo make this optional static dispatch for better performance
  // todo remove arc
  pub(crate) data: Arc<dyn ComponentStorage<T>>,
  /// watch this component all change with idx
  pub(crate) group_watchers: EventSource<IndexValueChange<T>>,
}

impl<T> ComponentCollection<T> {
  pub fn read(&self) -> ComponentReadView<T> {
    ComponentReadView {
      data: self.data.create_read_view(),
    }
  }
  pub fn write(&self) -> ComponentWriteView<T> {
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
  data: Box<dyn ComponentStorageReadView<T>>,
}

impl<T: 'static> ComponentReadView<T> {
  pub fn get(&self, idx: AllocIdx<T>) -> &T {
    self.data.get(idx.index as usize).unwrap()
  }
}

pub struct IndexValueChange<T> {
  pub idx: AllocIdx<T>,
  pub change: ValueChange<T>,
}

pub struct ComponentWriteView<T: 'static> {
  pub(crate) data: Box<dyn ComponentStorageReadWriteView<T>>,
  pub(crate) events: MutexGuardHolder<Source<IndexValueChange<T>>>,
}

impl<T: CValue + Default> ComponentWriteView<T> {
  pub fn with_writer(self, f: impl FnMut() -> T + 'static) -> impl EntityComponentWriter {
    EntityComponentWriterImpl {
      component: Some(self),
      default_value: f,
    }
  }

  pub fn write(&mut self, idx: AllocIdx<T>, new: T) {
    self.write_impl(idx, new, false);
  }

  pub(crate) fn write_impl(&mut self, idx: AllocIdx<T>, new: T, is_create: bool) {
    let com = self.data.get_mut(idx.index as usize).unwrap();
    let previous = std::mem::replace(com, new.clone());

    if is_create {
      let change = ValueChange::Delta(new, None);
      self.events.emit(&IndexValueChange { idx, change });
      return;
    }

    if previous == new {
      return;
    }

    let change = ValueChange::Delta(new, Some(previous));
    self.events.emit(&IndexValueChange { idx, change });
  }

  pub(crate) fn delete(&mut self, idx: AllocIdx<T>) {
    let com = self.data.get_mut(idx.index as usize).unwrap();
    let previous = std::mem::take(com);

    let change = ValueChange::Remove(previous);
    self.events.emit(&IndexValueChange { idx, change });
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
