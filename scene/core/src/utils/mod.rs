mod identity;
use std::sync::atomic::AtomicBool;

use futures::Stream;
pub use identity::*;
mod mapper;
pub use mapper::*;
mod scene_item;
pub use scene_item::*;

use crate::*;

pub enum Partial<'a, T: IncrementalBase> {
  All(&'a T),
  Delta(&'a T::Delta),
}

#[macro_export]
macro_rules! with_field {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      Partial::All(model) => send(model.$field.clone()),
      Partial::Delta(delta) => {
        if let DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

impl<T: IncrementalBase> SceneItemRef<T> {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(Partial<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    let inner = self.read();
    inner.listen_by(mapper)
  }
}

impl<T: IncrementalBase> Identity<T> {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(Partial<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    let sender_c = sender.clone();
    let send = move |mapped| {
      sender_c.unbounded_send(mapped).ok();
    };
    mapper(Partial::All(self), &send);

    self.delta_stream.on(move |v| {
      mapper(Partial::Delta(v.delta), &send);
      sender.is_closed()
    });
    receiver
  }
}

struct IdentitySignal<T: Incremental, F, U> {
  inner: std::sync::Weak<RwLock<Identity<T>>>,
  mapped: Option<U>,
  mapper: F,
  changed: Arc<AtomicBool>,
}

impl<T: Incremental, F, U> Stream for IdentitySignal<T, F, U> {
  type Item = U;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    if let Some(source) = self.inner.upgrade() {
      if self.changed {
        //
      } else {
        //
      }
    } else {
      std::task::Poll::Ready(None)
    }
  }
}
