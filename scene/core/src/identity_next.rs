use std::{
  ops::DerefMut,
  sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::*;

use incremental::Incremental;
use reactive::{EventDispatcher, Signal, StreamSignal};

pub struct SceneItemRef<T: Incremental> {
  inner: Arc<RwLock<Identity<T>>>,
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

  pub fn mutate<R>(&self, mut mutator: impl FnMut(&mut Identity<T>) -> R) -> R {
    let mut inner = self.inner.write().unwrap();
    let r = mutator(&mut inner);
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
  change_dispatcher: EventDispatcher<T::Delta>,
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

  pub fn id(&self) -> usize {
    self.id
  }
}

impl<T: Default + Incremental> Default for Identity<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: Incremental> Drop for Identity<T> {
  fn drop(&mut self) {
    self.drop_dispatcher.emit(&())
  }
}

impl<T: Incremental> std::ops::Deref for Identity<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T: Incremental> Identity<T> {
  pub fn mutate(&mut self, delta: T::Delta) {
    self.change_dispatcher.emit(&delta);
    self.inner.apply(delta).unwrap();
  }
}

pub struct IdentityMapper<T, U> {
  data: HashMap<usize, StreamSignal<T>>,
  phantom: PhantomData<U>,
}

impl<T, U: Incremental> IdentityMapper<T, U> {
  pub fn new(folder: impl Fn(U) -> T) -> Self {
    Self {
      data: Default::default(),
      phantom: Default::default(),
    }
  }
}

impl<T: 'static, U: 'static> IdentityMapper<T, U> {
  /// this to bypass the borrow limits of get_update_or_insert_with
  pub fn get_update_or_insert_with_logic<'a, 'b, X>(
    &'b mut self,
    source: &'a Identity<X>,
    mut logic: impl FnMut(ResourceLogic<'a, 'b, T, X>) -> ResourceLogicResult<'b, T>,
  ) -> &'b mut T {
    let mut new_created = false;
    let mut resource = self.data.entry(source.id).or_insert_with(|| {
      let value = logic(ResourceLogic::Create(&source.inner)).unwrap_new();
      new_created = true;

      source.drop_dispatcher.stream().on(|v| self.data.remove(k));

      let value_should_update = source.change_dispatcher.stream().map(|_| true).hold(true);

      Mapped {
        value,
        value_should_update,
      }
    });

    if new_created || resource.value_should_update.sample() {
      logic(ResourceLogic::Update(&mut resource.value, source)).unwrap_update();
    }

    &mut resource.value
  }

  /// direct function version
  pub fn get_update_or_insert_with<X>(
    &mut self,
    source: &Identity<X>,
    creator: impl FnOnce(&X) -> T,
    updater: impl FnOnce(&mut T, &X),
  ) -> &mut T {
    self.get_update_or_insert_with_logic(source, |logic| match logic {
      ResourceLogic::Create(x) => ResourceLogicResult::Create(creator(x)),
      ResourceLogic::Update(t, x) => {
        updater(t, x);
        ResourceLogicResult::Update(t)
      }
    })
  }

  pub fn get_unwrap<X>(&self, source: &Identity<X>) -> &T {
    &self.data.get(&source.id).unwrap().value
  }
}
