use std::sync::{Arc, RwLock, Weak};

use crate::*;

pub struct ReactiveIncrementalStorage<T: Incremental> {
  inner: Arc<ReactiveIncrementalStorageImpl<T>>,
}

pub struct ReactiveIncrementalStorageImpl<T: Incremental> {
  data: RwLock<Vec<T>>,
  deltas: RwLock<Vec<T::Delta>>,
  sub_listeners: RwLock<Vec<Box<dyn FnMut(&T) -> bool + Send + Sync>>>,
  sub_listener_mapping: RwLock<Vec<(usize, usize)>>,
  group_listeners: RwLock<Vec<Box<dyn FnMut(&[T::Delta]) -> bool + Send + Sync>>>,
}

impl<T: Incremental> ReactiveIncrementalStorageImpl<T> {
  pub fn allocate(&self, data: T) -> IncrementalSignalPtr<T> {
    todo!()
  }

  pub fn on(&self, cb: impl FnMut(&[T]) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    todo!()
  }
  pub fn off(&self, token: RemoveToken<T>) {
    todo!()
  }
}

pub struct IncrementalSignalPtr<T: Incremental> {
  idx: usize,
  source: Weak<ReactiveIncrementalStorageImpl<T>>,
}

impl<T: Incremental> IncrementalSignalPtr<T> {
  pub fn on(&self, cb: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    todo!()
  }
  pub fn off(&self, token: RemoveToken<T>) {
    todo!()
  }

  pub fn mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    todo!()
  }
}
