use std::{
  ops::DerefMut,
  sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::*;

use reactive::{EventDispatcher, Stream, StreamSignal};

pub struct SceneItemRef<T: Incremental> {
  inner: Arc<RwLock<Identity<T>>>,
}

impl<T: Incremental + Send + Sync> SimpleIncremental for SceneItemRef<T> {
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

pub struct Mutating<'a, T: Incremental> {
  pub inner: &'a mut T,
  pub collector: &'a mut dyn FnMut(&T, &T::Delta),
}

impl<'a, T: Incremental> Deref for Mutating<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.inner
  }
}

impl<'a, T: Incremental> Mutating<'a, T> {
  pub fn modify(&mut self, delta: T::Delta) {
    (self.collector)(self.inner, &delta);
    self.inner.apply(delta).unwrap()
  }

  pub fn trigger_manual(&mut self, delta: T::Delta) {
    (self.collector)(self.inner, &delta);
  }
}

impl<T: Incremental> SceneItemRef<T> {
  pub fn new(source: T) -> Self {
    let inner = Arc::new(RwLock::new(Identity::new(source)));
    Self { inner }
  }

  pub fn mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    let mut inner = self.inner.write().unwrap();
    let i: &mut Identity<T> = &mut inner;
    i.mutate(mutator)
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
  change_dispatcher: EventDispatcher<DeltaView<'static, T>>,
  drop_dispatcher: EventDispatcher<()>,
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
      change_dispatcher: Default::default(),
      drop_dispatcher: Default::default(),
    }
  }

  pub fn delta_stream(&self) -> Stream<DeltaView<'static, T>> {
    self.change_dispatcher.stream()
  }

  pub fn id(&self) -> usize {
    self.id
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    let data = &mut self.inner;
    let dispatcher = &self.change_dispatcher;
    mutator(Mutating {
      inner: data,
      collector: &mut |data, delta| {
        let view = DeltaView { data, delta };
        let view = unsafe { std::mem::transmute(view) };
        dispatcher.emit(&view);
      },
    })
  }
}

impl<T: Default + Incremental> Default for Identity<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: Incremental> Drop for Identity<T> {
  fn drop(&mut self) {
    self.drop_dispatcher.emit(&());
  }
}

impl<T: Incremental> std::ops::Deref for Identity<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

/// A reactive map container
pub struct IdentityMapper<T, U: Incremental> {
  data: Arc<RwLock<HashMap<usize, StreamSignal<T>>>>,
  phantom: PhantomData<U>,
  // refresher
  updater: Arc<dyn Fn(&U, &U::Delta, &mut T) + Send + Sync>,
  creator: Box<dyn Fn(&U) -> T>,
}

impl<T: Send + Sync + 'static, U: Incremental> IdentityMapper<T, U> {
  pub fn insert(&mut self, source: &Identity<U>) {
    let id = source.id;

    let data = Arc::downgrade(&self.data);
    source.drop_dispatcher.stream().on(move |_| {
      if let Some(data) = data.upgrade() {
        let mut data = data.write().unwrap();
        data.remove(&id);
      }
      true
    });

    let updater = self.updater.clone();
    self.data.write().unwrap().entry(id).or_insert_with(|| {
      source
        .change_dispatcher
        .stream()
        .fold((self.creator)(source), move |view, mapped| {
          updater(view.data, view.delta, mapped);
          false
        })
    });
  }

  pub fn remove(&mut self, source: &Identity<U>) {
    self.data.write().unwrap().remove(&source.id);
  }

  // pub fn get(&mut self, source: &Identity<U>) {
  //   self.data.remove(&source.id);
  // }
}
