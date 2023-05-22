use std::sync::atomic::{AtomicUsize, Ordering};

use reactive::{EventOnceSource, EventSource};

use super::scene_item::Mutating;
use crate::*;

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub fn alloc_global_res_id() -> usize {
  GLOBAL_ID.fetch_add(1, Ordering::Relaxed)
}

pub trait GlobalIdentified {
  fn guid(&self) -> usize;
}
define_dyn_trait_downcaster_static!(GlobalIdentified);

pub struct Identity<T: IncrementalBase> {
  pub(super) id: usize,
  pub(super) inner: T,
  pub delta_source: EventSource<T::Delta>,
  pub drop_source: EventOnceSource<()>,
}

impl<T: IncrementalBase> AsRef<T> for Identity<T> {
  fn as_ref(&self) -> &T {
    &self.inner
  }
}

impl<T: IncrementalBase> From<T> for Identity<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

trait ModifyIdentityDelta<T: Incremental> {
  fn apply(self, target: &mut Identity<T>);
}

impl<T, X> ModifyIdentityDelta<T> for X
where
  T: Incremental<Delta = X>,
{
  fn apply(self, target: &mut Identity<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}

impl<T: IncrementalBase> GlobalIdentified for Identity<T> {
  fn guid(&self) -> usize {
    self.id
  }
}
impl<T: IncrementalBase> AsRef<dyn GlobalIdentified> for Identity<T> {
  fn as_ref(&self) -> &(dyn GlobalIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn GlobalIdentified> for Identity<T> {
  fn as_mut(&mut self) -> &mut (dyn GlobalIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> Identity<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      id: alloc_global_res_id(),
      delta_source: Default::default(),
      drop_source: Default::default(),
    }
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    self.mutate_with(mutator, |_| {})
  }

  pub fn mutate_with<R>(
    &mut self,
    mutator: impl FnOnce(Mutating<T>) -> R,
    mut extra_collector: impl FnMut(T::Delta),
  ) -> R {
    let data = &mut self.inner;
    let dispatcher = &self.delta_source;
    mutator(Mutating {
      inner: data,
      collector: &mut |_, delta| {
        dispatcher.emit(delta);
        extra_collector(delta.clone())
      },
    })
  }
}

impl<T: Default + IncrementalBase> Default for Identity<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: IncrementalBase> Drop for Identity<T> {
  fn drop(&mut self) {
    self.drop_source.emit(&());
  }
}

impl<T: IncrementalBase> std::ops::Deref for Identity<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
