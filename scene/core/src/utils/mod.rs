mod identity;
pub use identity::*;
mod mapper;
pub use mapper::*;
mod scene_item;
pub use scene_item::*;

use futures::Future;

use crate::*;

pub enum Partial<'a, T: IncrementalBase> {
  All(&'a T),
  Delta(&'a T::Delta),
}

#[macro_export]
macro_rules! with_field {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      Partial::All(value) => send(value.$field.clone()),
      Partial::Delta(delta) => {
        if let DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

#[macro_export]
macro_rules! with_field_change {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      Partial::All(value) => send(()),
      Partial::Delta(delta) => {
        if let DeltaOf::<$ty>::$field(field) = delta {
          send(())
        }
      }
    }
  };
}

pub fn all_delta<T: Incremental>(view: Partial<T>, send: &dyn Fn(T::Delta)) {
  match view {
    Partial::All(value) => value.expand(send),
    Partial::Delta(delta) => send(delta.clone()),
  }
}

pub fn any_change<T: Incremental>(view: Partial<T>, send: &dyn Fn(())) {
  match view {
    Partial::All(_) => send(()),
    Partial::Delta(_) => send(()),
  }
}

impl<T: IncrementalBase> SceneItemRef<T> {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(Partial<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    let inner = self.read();
    inner.listen_by(mapper)
  }

  pub fn create_drop(&self) -> impl Future<Output = ()> {
    let inner = self.read();
    inner.create_drop()
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

    self.delta_source.on(move |v| {
      mapper(Partial::Delta(v.delta), &send);
      // todo, check if receiver drop logic?
      sender.is_closed()
    });
    // todo impl custom unbound channel: if sender drop, the receiver will still hold the history message
    // which is unnecessary. The better behavior will just drop the history and emit Poll::Ready::None

    // todo impl single value channel, and history compactor (synchronous version)
    receiver
  }

  // todo, how to handle too many drop listener? in fact we never cleanup them
  pub fn create_drop(&self) -> impl Future<Output = ()> {
    let (sender, receiver) = futures::channel::oneshot::channel::<()>();
    self.drop_source.on(move |_| {
      sender.send(()).ok();
    });
    use futures::FutureExt;
    receiver.map(|_| ())
  }
}
