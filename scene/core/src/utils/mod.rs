mod identity;

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

  // pub fn listen_by_single<U: Send + Sync + 'static>(
  //   &self,
  //   mapper: impl Fn(Partial<T>) -> Option<U> + Send + Sync + 'static,
  // ) -> impl futures::Stream<Item = U> {
  //   let inner = self.read();
  //   inner.listen_by_single(mapper)
  // }
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
      // todo, check if receiver drop logic?
      sender.is_closed()
    });
    // todo impl custom unbound channel: if sender drop, the receiver will still hold the history message
    // which is unnecessary. The better behavior will just drop the history and emit Poll::Ready::None
    receiver
  }

  // pub fn listen_by_single<U: Send + Sync + 'static>(
  //   &self,
  //   mapper: impl Fn(Partial<T>) -> Option<U> + Send + Sync + 'static,
  // ) -> impl futures::Stream<Item = U> {
  //   // todo use one value channel
  // }
}

// struct IdentitySignal<T: Incremental, F, U> {
//   inner: std::sync::Weak<RwLock<Identity<T>>>,
//   mapped: Option<U>,
//   mapper: F,
//   changed: Arc<AtomicBool>,
// }

// impl<T, F, U> Stream for IdentitySignal<T, F, U>
// where
//   T: Incremental,
//   F: Fn(&T) -> U,
// {
//   type Item = U;

//   fn poll_next(
//     self: std::pin::Pin<&mut Self>,
//     _cx: &mut std::task::Context<'_>, // todo, we do not rely on weaker
//   ) -> std::task::Poll<Option<Self::Item>> {
//     if let Some(source) = self.inner.upgrade() {
//       if self
//         .changed
//         .swap(false, std::sync::atomic::Ordering::SeqCst)
//       {
//         let mapper = unsafe { self.map_unchecked_mut(|v| &mut v.mapper) };
//         let source = source.read().unwrap();
//         let new = mapper(&source);
//         std::task::Poll::Ready(Some(new))
//       } else {
//         std::task::Poll::Pending
//       }
//     } else {
//       std::task::Poll::Ready(None)
//     }
//   }
// }
