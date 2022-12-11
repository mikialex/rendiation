use std::sync::atomic::{AtomicUsize, Ordering};

use incremental::{DeltaView, Incremental};
use reactive::{EventDispatcher, Stream};

use super::scene_item::Mutating;

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Identity<T: Incremental> {
  pub(super) id: usize,
  pub(super) inner: T,
  pub change_dispatcher: EventDispatcher<DeltaView<'static, T>>,
  pub drop_dispatcher: EventDispatcher<()>,
}

impl<T: Incremental> AsRef<T> for Identity<T> {
  fn as_ref(&self) -> &T {
    &self.inner
  }
}

impl<T: Incremental> From<T> for Identity<T> {
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
