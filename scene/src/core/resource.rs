use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  marker::PhantomData,
  rc::Rc,
  sync::atomic::{AtomicUsize, Ordering},
};

use arena::Arena;

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub struct ResourceWrapped<T> {
  id: usize,
  inner: T,
  pub watchers: Arena<Box<dyn Watcher<T>>>,
}

impl<T> ResourceWrapped<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      id: GLOBAL_ID.fetch_add(1, Ordering::Relaxed),
      watchers: Default::default(),
    }
  }

  pub fn trigger_change(&mut self) {
    self
      .watchers
      .iter_mut()
      .for_each(|(_, w)| w.will_change(&self.inner, self.id));
  }
}

impl<T: Default> Default for ResourceWrapped<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T> Drop for ResourceWrapped<T> {
  fn drop(&mut self) {
    self
      .watchers
      .iter_mut()
      .for_each(|(_, w)| w.will_drop(&self.inner, self.id));
  }
}

impl<T> std::ops::Deref for ResourceWrapped<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> std::ops::DerefMut for ResourceWrapped<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.trigger_change();
    &mut self.inner
  }
}

pub trait Watcher<T> {
  fn will_change(&mut self, item: &T, id: usize);
  fn will_drop(&mut self, item: &T, id: usize);
}

pub struct ResourceMapper<T, U> {
  data: HashMap<usize, T>,
  to_remove: Rc<RefCell<Vec<usize>>>,
  changed: Rc<RefCell<HashSet<usize>>>,
  phantom: PhantomData<U>,
}

impl<T, U> Default for ResourceMapper<T, U> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      to_remove: Default::default(),
      changed: Default::default(),
      phantom: Default::default(),
    }
  }
}

impl<T, U> ResourceMapper<T, U> {
  pub fn maintain(&mut self) {
    self.to_remove.borrow_mut().drain(..).for_each(|id| {
      self.data.remove(&id);
    });
  }

  pub fn get_update_or_insert_with(
    &mut self,
    source: &mut ResourceWrapped<U>,
    creator: impl FnOnce(&U) -> T,
    updater: impl FnOnce(&mut T, &U),
  ) -> &mut T {
    let mut new_created = false;
    let resource = self.data.entry(source.id).or_insert_with(|| {
      let item = creator(&source.inner);
      new_created = true;
      source
        .watchers
        .insert(Box::new(ResourceWatcherWithAutoClean {
          to_remove: self.to_remove.clone(),
          changed: self.changed.clone(),
        }));
      item
    });

    if new_created || self.changed.borrow_mut().remove(&source.id) {
      updater(resource, source)
    }

    resource
  }

  pub fn get_unwrap(&self, source: &ResourceWrapped<U>) -> &T {
    self.data.get(&source.id).unwrap()
  }
}

struct ResourceWatcherWithAutoClean {
  to_remove: Rc<RefCell<Vec<usize>>>,
  changed: Rc<RefCell<HashSet<usize>>>,
}

impl<T> Watcher<T> for ResourceWatcherWithAutoClean {
  fn will_change(&mut self, _camera: &T, id: usize) {
    self.changed.borrow_mut().insert(id);
  }

  fn will_drop(&mut self, _camera: &T, id: usize) {
    self.to_remove.borrow_mut().push(id);
  }
}
