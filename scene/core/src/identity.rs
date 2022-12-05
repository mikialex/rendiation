use std::{
  ops::DerefMut,
  sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::*;

use arena::Arena;

pub struct SceneItemRef<T: Incremental> {
  inner: Arc<RwLock<Identity<T>>>,
}

impl<T: Incremental> SimpleIncremental for SceneItemRef<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta;
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

impl<T: Incremental> Clone for SceneItemRef<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}
impl<T: Incremental> From<T> for SceneItemRef<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

impl<T: Incremental> SceneItemRef<T> {
  pub fn new(source: T) -> Self {
    let inner = Arc::new(RwLock::new(Identity::new(source)));
    Self { inner }
  }

  pub fn mutate<R>(&self, mut mutator: impl FnMut(&mut T) -> R) -> R {
    let mut inner = self.inner.write().unwrap();
    let r = mutator(&mut inner);
    inner.trigger_change();
    r
  }
  pub fn visit<R>(&self, mut visitor: impl FnMut(&T) -> R) -> R {
    let inner = self.inner.read().unwrap();
    visitor(&inner)
  }

  pub fn read(&self) -> SceneItemRefGuard<T> {
    self
      .inner
      .read()
      .ok()
      .map(|inner| SceneItemRefGuard { inner })
      .unwrap()
  }
  pub fn write(&self) -> SceneItemRefMutGuard<T> {
    self
      .inner
      .write()
      .ok()
      .map(|inner| SceneItemRefMutGuard { inner })
      .unwrap()
  }
}

pub struct SceneItemRefGuard<'a, T: Incremental> {
  inner: RwLockReadGuard<'a, Identity<T>>,
}

impl<'a, T: Incremental> Deref for SceneItemRefGuard<'a, T> {
  type Target = Identity<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

pub struct SceneItemRefMutGuard<'a, T: Incremental> {
  inner: RwLockWriteGuard<'a, Identity<T>>,
}

impl<'a, T: Incremental> Deref for SceneItemRefMutGuard<'a, T> {
  type Target = Identity<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

impl<'a, T: Incremental> DerefMut for SceneItemRefMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.deref_mut()
  }
}

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Identity<T: Incremental> {
  id: usize,
  inner: T,
  pub watchers: RwLock<Arena<Box<dyn Watcher<T>>>>,
}

impl<T: Incremental> AsRef<T> for Identity<T> {
  fn as_ref(&self) -> &T {
    &self.inner
  }
}

pub trait IntoSceneItemRef: Sized + Incremental {
  fn into_ref(self) -> SceneItemRef<Self> {
    self.into()
  }
}

impl<T: Incremental> IntoSceneItemRef for T {}

impl<T: Incremental> From<T> for Identity<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

impl<T: Incremental> Identity<T> {
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

impl<T: Default + Incremental> Default for Identity<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: Incremental> Drop for Identity<T> {
  fn drop(&mut self) {
    self
      .watchers
      .write()
      .unwrap()
      .iter_mut()
      .for_each(|(_, w)| w.will_drop(&self.inner, self.id));
  }
}

impl<T: Incremental> std::ops::Deref for Identity<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T: Incremental> std::ops::DerefMut for Identity<T> {
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

pub struct IdentityMapper<T, U: ?Sized> {
  data: HashMap<usize, T>,
  to_remove: Arc<RwLock<Vec<usize>>>,
  changed: Arc<RwLock<HashSet<usize>>>,
  phantom: PhantomData<U>,
}

impl<T, U: ?Sized> Default for IdentityMapper<T, U> {
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

pub trait RequireMaintain: std::any::Any {
  fn maintain(&mut self);
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: 'static, U: 'static + ?Sized> RequireMaintain for IdentityMapper<T, U> {
  fn maintain(&mut self) {
    self.to_remove.write().unwrap().drain(..).for_each(|id| {
      self.data.remove(&id);
    });
  }
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

impl<T: 'static, U: 'static + ?Sized> IdentityMapper<T, U> {
  /// this to bypass the borrow limits of get_update_or_insert_with
  pub fn get_update_or_insert_with_logic<'a, 'b, X: Incremental>(
    &'b mut self,
    source: &'a Identity<X>,
    mut logic: impl FnMut(ResourceLogic<'a, 'b, T, X>) -> ResourceLogicResult<'b, T>,
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

  pub fn get_update_or_insert_with<X: Incremental>(
    &mut self,
    source: &Identity<X>,
    creator: impl FnOnce(&X) -> T,
    updater: impl FnOnce(&mut T, &X),
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
      updater(resource, &source.inner)
    }

    resource
  }

  pub fn get_unwrap<X: Incremental>(&self, source: &Identity<X>) -> &T {
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
