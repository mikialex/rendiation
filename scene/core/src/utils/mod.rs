mod identity;
pub use identity::*;
mod scene_item;
pub use scene_item::*;
mod transformation;
use futures::Future;
use reactive::{
  ChannelLike, DefaultSingleValueChannel, DefaultUnboundChannel, EventSourceDropper,
  EventSourceStream,
};
pub use transformation::*;

use crate::*;

#[macro_export]
macro_rules! with_field {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => send(value.$field.clone()),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

#[macro_export]
macro_rules! with_field_expand {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => value.$field.expand(send),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
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
      incremental::MaybeDeltaRef::All(value) => send(()),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(())
        }
      }
    }
  };
}

pub fn all_delta<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(T::Delta)) {
  all_delta_with(true, Some)(view, send)
}

pub fn all_delta_no_init<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(T::Delta)) {
  all_delta_with(false, Some)(view, send)
}

pub fn any_change<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(())) {
  any_change_with(true)(view, send)
}

pub fn any_change_no_init<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(())) {
  any_change_with(false)(view, send)
}

pub fn no_change<T: IncrementalBase>(_view: MaybeDeltaRef<T>, _send: &dyn Fn(())) {
  // do nothing at all
}

#[inline(always)]
pub fn any_change_with<T: IncrementalBase>(
  should_send_when_init: bool,
) -> impl Fn(MaybeDeltaRef<T>, &dyn Fn(())) {
  move |view, send| match view {
    MaybeDeltaRef::All(_) => {
      if should_send_when_init {
        send(())
      }
    }
    MaybeDeltaRef::Delta(_) => send(()),
  }
}

#[inline(always)]
pub fn all_delta_with<T: IncrementalBase, X>(
  should_send_when_init: bool,
  filter_map: impl Fn(T::Delta) -> Option<X>,
) -> impl Fn(MaybeDeltaRef<T>, &dyn Fn(X)) {
  move |view, send| {
    let my_send = |d| {
      if let Some(d) = filter_map(d) {
        send(d)
      }
    };
    match view {
      MaybeDeltaRef::All(value) => {
        if should_send_when_init {
          value.expand(my_send)
        }
      }
      MaybeDeltaRef::Delta(delta) => my_send(delta.clone()),
    }
  }
}

impl<T: IncrementalBase> SceneItemRef<T> {
  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    let inner = self.read();
    inner.listen_by::<DefaultUnboundChannel, _>(mapper)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    let inner = self.read();
    inner.listen_by::<DefaultSingleValueChannel, _>(mapper)
  }

  pub fn listen_by<C, U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    C: ChannelLike<U>,
    U: Send + Sync + 'static,
  {
    let inner = self.read();
    inner.listen_by::<C, _>(mapper)
  }

  pub fn create_drop(&self) -> impl Future<Output = ()> {
    let inner = self.read();
    inner.create_drop()
  }
}

impl<T: IncrementalBase> Identity<T> {
  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<DefaultUnboundChannel, _>(mapper)
  }

  pub fn listen_by<C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U>,
  {
    let (sender, receiver) = C::build();
    let sender_c = sender.clone();
    let send = move |mapped| {
      C::send(&sender_c, mapped);
    };
    mapper(MaybeDeltaRef::All(self), &send);

    let remove_token = self.delta_source.on(move |v| {
      mapper(MaybeDeltaRef::Delta(v), &send);
      C::is_closed(&sender)
    });
    // todo impl custom unbound channel: if sender drop, the receiver will still hold the history
    // message which is unnecessary. The better behavior will just drop the history and emit
    // Poll::Ready::None

    let dropper = EventSourceDropper::new(remove_token, self.delta_source.make_weak());
    EventSourceStream::new(dropper, receiver)
  }

  pub fn create_drop(&self) -> impl Future<Output = ()> {
    use futures::FutureExt;
    self
      .listen_by::<DefaultSingleValueChannel, _>(no_change)
      .count()
      .map(|_| {})
  }
}

#[test]
fn channel_behavior() {
  // we rely on this behavior to cleanup the sender callback
  {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();
    drop(receiver);
    assert!(sender.is_closed())
  }

  // this is why we should impl custom channel
  {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();
    sender.unbounded_send(999).ok();
    sender.unbounded_send(9999).ok();
    drop(sender);

    let all = futures::executor::block_on_stream(receiver).count();

    assert_eq!(all, 2)
  }

  // should wake when drop sender
  {
    use std::sync::atomic::AtomicBool;

    struct TestWaker {
      waked: Arc<AtomicBool>,
    }

    impl futures::task::ArcWake for TestWaker {
      fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self
          .waked
          .store(true, std::sync::atomic::Ordering::SeqCst);
      }
    }

    {
      let (sender, mut receiver) = futures::channel::mpsc::unbounded::<usize>();

      let test_waked = Arc::new(AtomicBool::new(false));
      let waker = Arc::new(TestWaker {
        waked: test_waked.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx = std::task::Context::from_waker(&waker);

      // install waker
      use futures::StreamExt;
      let _ = receiver.poll_next_unpin(&mut cx);

      drop(sender);

      let waked = test_waked.load(std::sync::atomic::Ordering::SeqCst);
      assert!(waked);
    }
  }
}
