use crate::*;
use reactive::{EventOnceSource, EventSource};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::scene_item::Mutating;

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Identity<T: IncrementalBase> {
  pub(super) id: usize,
  pub(super) inner: T,
  pub delta_source: EventSource<DeltaView<'static, T>>,
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

impl<T: IncrementalBase> Identity<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      id: GLOBAL_ID.fetch_add(1, Ordering::Relaxed),
      delta_source: Default::default(),
      drop_source: Default::default(),
    }
  }

  pub fn id(&self) -> usize {
    self.id
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    let data = &mut self.inner;
    let dispatcher = &self.delta_source;
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
