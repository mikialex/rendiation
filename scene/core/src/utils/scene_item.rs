use std::{
  ops::{Deref, DerefMut},
  sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::*;

use super::identity::Identity;

#[derive(Default)]
pub struct SceneItemRef<T: IncrementalBase> {
  inner: Arc<RwLock<Identity<T>>>,
}

impl<T: IncrementalBase + Send + Sync> IncrementalBase for SceneItemRef<T> {
  type Delta = Self;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

impl<T: ApplicableIncremental + Send + Sync> ApplicableIncremental for SceneItemRef<T> {
  type Error = T::Error;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    *self = delta;
    Ok(())
  }
}

impl<T: IncrementalBase> Clone for SceneItemRef<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}
impl<T: IncrementalBase> From<T> for SceneItemRef<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

pub struct Mutating<'a, T: IncrementalBase> {
  pub inner: &'a mut T,
  pub collector: &'a mut dyn FnMut(&T, &T::Delta),
}

impl<'a, T: IncrementalBase> Deref for Mutating<'a, T> {
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
}

impl<'a, T: IncrementalBase> Mutating<'a, T> {
  pub fn trigger_manual(&mut self, modify: impl FnOnce(&mut T) -> T::Delta) {
    let delta = modify(self.inner);
    (self.collector)(self.inner, &delta);
  }
}

pub trait ModifySceneItemDelta<T: IncrementalBase> {
  fn apply_modify(self, target: &SceneItemRef<T>);
}

impl<T, X> ModifySceneItemDelta<T> for X
where
  T: Incremental<Delta = X>,
{
  fn apply_modify(self, target: &SceneItemRef<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}

impl<T: IncrementalBase> SceneItemRef<T> {
  pub fn new(source: T) -> Self {
    let inner = Arc::new(RwLock::new(Identity::new(source)));
    Self { inner }
  }

  pub fn mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    // ignore lock poison
    let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
    let i: &mut Identity<T> = &mut inner;
    i.mutate(mutator)
  }
  pub fn visit<R>(&self, mut visitor: impl FnMut(&T) -> R) -> R {
    // ignore lock poison
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    visitor(&inner)
  }

  pub fn read(&self) -> SceneItemRefGuard<T> {
    // ignore lock poison
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    SceneItemRefGuard { inner }
  }
}

pub struct SceneItemRefGuard<'a, T: IncrementalBase> {
  inner: RwLockReadGuard<'a, Identity<T>>,
}

impl<'a, T: IncrementalBase> Deref for SceneItemRefGuard<'a, T> {
  type Target = Identity<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

pub struct SceneItemRefMutGuard<'a, T: IncrementalBase> {
  inner: RwLockWriteGuard<'a, Identity<T>>,
}

impl<'a, T: IncrementalBase> Deref for SceneItemRefMutGuard<'a, T> {
  type Target = Identity<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

impl<'a, T: IncrementalBase> DerefMut for SceneItemRefMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.deref_mut()
  }
}

pub trait IntoSceneItemRef: Sized + IncrementalBase {
  fn into_ref(self) -> SceneItemRef<Self> {
    self.into()
  }
}

impl<T: IncrementalBase> IntoSceneItemRef for T {}
