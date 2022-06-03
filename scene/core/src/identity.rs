use crate::*;

use arena::Arena;

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Identity<T> {
  id: usize,
  inner: T,
  pub watchers: RwLock<Arena<Box<dyn Watcher<T>>>>,
}

pub trait IntoResourced: Sized {
  fn into_resourced(self) -> Identity<Self> {
    self.into()
  }
}

impl<T> IntoResourced for T {}

impl<T> From<T> for Identity<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

impl<T> Identity<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      id: GLOBAL_ID.fetch_add(1, Ordering::Relaxed),
      watchers: Default::default(),
    }
  }

  pub fn id(&self) -> usize {
    self.id
  }

  pub fn trigger_change(&mut self) {
    let mut to_drop = Vec::with_capacity(0);
    self
      .watchers
      .write()
      .unwrap()
      .iter_mut()
      .for_each(|(h, w)| {
        if !w.will_change(&self.inner, self.id) {
          to_drop.push(h)
        }
      });

    for handle in to_drop.drain(..) {
      self.watchers.write().unwrap().remove(handle);
    }
  }
}

impl<T: Default> Default for Identity<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T> Drop for Identity<T> {
  fn drop(&mut self) {
    self
      .watchers
      .write()
      .unwrap()
      .iter_mut()
      .for_each(|(_, w)| w.will_drop(&self.inner, self.id));
  }
}

impl<T> std::ops::Deref for Identity<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> std::ops::DerefMut for Identity<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.trigger_change();
    &mut self.inner
  }
}

pub trait Watcher<T>: Sync + Send {
  // return should continue watch
  fn will_change(&mut self, item: &T, id: usize) -> bool;
  fn will_drop(&mut self, item: &T, id: usize);
}

pub struct IdentityMapper<T, U> {
  data: HashMap<usize, T>,
  to_remove: Arc<RwLock<Vec<usize>>>,
  changed: Arc<RwLock<HashSet<usize>>>,
  phantom: PhantomData<U>,
}

impl<T, U> Default for IdentityMapper<T, U> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      to_remove: Default::default(),
      changed: Default::default(),
      phantom: Default::default(),
    }
  }
}

pub enum ResourceLogic<'a, 'b, T, U> {
  Create(&'a U),
  Update(&'b mut T, &'a U),
}
pub enum ResourceLogicResult<'a, T> {
  Create(T),
  Update(&'a mut T),
}

impl<'a, T> ResourceLogicResult<'a, T> {
  pub fn unwrap_new(self) -> T {
    match self {
      ResourceLogicResult::Create(v) => v,
      ResourceLogicResult::Update(_) => panic!(),
    }
  }

  pub fn unwrap_update(self) -> &'a mut T {
    match self {
      ResourceLogicResult::Create(_) => panic!(),
      ResourceLogicResult::Update(v) => v,
    }
  }
}

impl<T: 'static, U: 'static> IdentityMapper<T, U> {
  pub fn maintain(&mut self) {
    self.to_remove.write().unwrap().drain(..).for_each(|id| {
      self.data.remove(&id);
    });
  }

  /// this to bypass the borrow limits of get_update_or_insert_with
  pub fn get_update_or_insert_with_logic<'a, 'b>(
    &'b mut self,
    source: &'a Identity<U>,
    mut logic: impl FnMut(ResourceLogic<'a, 'b, T, U>) -> ResourceLogicResult<'b, T>,
  ) -> &'b mut T {
    let mut new_created = false;
    let mut resource = self.data.entry(source.id).or_insert_with(|| {
      let item = logic(ResourceLogic::Create(&source.inner)).unwrap_new();
      new_created = true;
      source
        .watchers
        .write()
        .unwrap()
        .insert(Box::new(ResourceWatcherWithAutoClean {
          to_remove: self.to_remove.clone(),
          changed: self.changed.clone(),
        }));
      item
    });

    if new_created || self.changed.write().unwrap().remove(&source.id) {
      resource = logic(ResourceLogic::Update(resource, source)).unwrap_update();
    }

    resource
  }

  pub fn get_update_or_insert_with(
    &mut self,
    source: &Identity<U>,
    creator: impl FnOnce(&U) -> T,
    updater: impl FnOnce(&mut T, &U),
  ) -> &mut T {
    let mut new_created = false;
    let resource = self.data.entry(source.id).or_insert_with(|| {
      let item = creator(&source.inner);
      new_created = true;
      source
        .watchers
        .write()
        .unwrap()
        .insert(Box::new(ResourceWatcherWithAutoClean {
          to_remove: self.to_remove.clone(),
          changed: self.changed.clone(),
        }));
      item
    });

    if new_created || self.changed.write().unwrap().remove(&source.id) {
      updater(resource, source)
    }

    resource
  }

  pub fn get_unwrap(&self, source: &Identity<U>) -> &T {
    self.data.get(&source.id).unwrap()
  }
}

struct ResourceWatcherWithAutoClean {
  to_remove: Arc<RwLock<Vec<usize>>>,
  changed: Arc<RwLock<HashSet<usize>>>,
}

impl<T> Watcher<T> for ResourceWatcherWithAutoClean {
  fn will_change(&mut self, _camera: &T, id: usize) -> bool {
    self.changed.write().unwrap().insert(id);
    true
  }

  fn will_drop(&mut self, _camera: &T, id: usize) {
    self.to_remove.write().unwrap().push(id);
  }
}
