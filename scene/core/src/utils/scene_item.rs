use std::{
  ops::{Deref, DerefMut},
  sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::*;

use super::identity::Identity;

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

pub trait ModifySceneItemDelta<T: Incremental> {
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

pub trait IntoSceneItemRef: Sized + Incremental {
  fn into_ref(self) -> SceneItemRef<Self> {
    self.into()
  }
}

impl<T: Incremental> IntoSceneItemRef for T {}
