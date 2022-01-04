use std::{
  cell::RefCell,
  collections::HashMap,
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
    self
      .watchers
      .iter_mut()
      .for_each(|(_, w)| w.will_change(&self.inner, self.id));
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
  phantom: PhantomData<U>,
}

impl<T, U> Default for ResourceMapper<T, U> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      to_remove: Default::default(),
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

  pub fn get_or_insert_with<W: FnMut(&U, usize) + 'static>(
    &mut self,
    source: &mut ResourceWrapped<U>,
    creator: impl FnOnce(&U) -> (T, W),
  ) -> &mut T {
    self.data.entry(source.id).or_insert_with(|| {
      let (item, w) = creator(&source.inner);
      source
        .watchers
        .insert(Box::new(ResourceWatcherWithAutoClean {
          to_remove: self.to_remove.clone(),
          watch: w,
        }));
      item
    })
  }

  pub fn get_unwrap(&self, source: &ResourceWrapped<U>) -> &T {
    self.data.get(&source.id).unwrap()
  }
}

struct ResourceWatcherWithAutoClean<W> {
  to_remove: Rc<RefCell<Vec<usize>>>,
  watch: W,
}

impl<T, W: FnMut(&T, usize)> Watcher<T> for ResourceWatcherWithAutoClean<W> {
  fn will_change(&mut self, camera: &T, id: usize) {
    (self.watch)(camera, id);
  }

  fn will_drop(&mut self, _camera: &T, id: usize) {
    self.to_remove.borrow_mut().push(id);
  }
}
