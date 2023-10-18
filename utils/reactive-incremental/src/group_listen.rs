use std::{
  sync::{Arc, Mutex, Weak},
  task::Waker,
};

use fast_hash_collection::*;

use crate::*;

impl<T: IncrementalBase + Clone> IncrementalSignalStorage<T> {
  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(&StorageGroupChange<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> impl Stream<Item = N> + Unpin
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    {
      let data = self.inner.data.write();

      for (index, data) in data.iter() {
        mapper(
          &StorageGroupChange::Create {
            data: unsafe { std::mem::transmute(data) },
            index,
          },
          &|mapped| {
            C::send(&sender, mapped);
          },
        )
      }
    }

    // could we try another way to do workaround this??
    let s: &'static Self = unsafe { std::mem::transmute(self) };

    let remove_token = s.on(move |v| {
      mapper(v, &|mapped| {
        C::send(&sender, mapped);
      });
      C::is_closed(&sender)
    });

    let dropper = EventSourceDropper::new(remove_token, self.inner.group_watchers.make_weak());
    DropperAttachedStream::new(dropper, receiver)
  }
}

pub struct GroupSingleValueSender<T> {
  inner: Weak<Mutex<(FastHashMap<u32, T>, Option<Waker>)>>,
}

impl<T> Drop for GroupSingleValueSender<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      let inner = inner.lock().unwrap();
      if let Some(waker) = &inner.1 {
        waker.wake_by_ref()
      }
    }
  }
}

pub struct GroupSingleValueReceiver<T> {
  inner: Arc<Mutex<(FastHashMap<u32, T>, Option<Waker>)>>,
}

impl<T> Stream for GroupSingleValueReceiver<T> {
  type Item = Vec<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    if let Ok(mut inner) = self.inner.lock() {
      inner.1 = cx.waker().clone().into();
      // check is_some first to avoid unnecessary move
      if !inner.0.is_empty() {
        let value = std::mem::take(&mut inner.0);
        Poll::Ready(Some(todo!()))
        // check if sender has dropped
      } else if Arc::weak_count(&self.inner) == 0 {
        Poll::Ready(None)
      } else {
        Poll::Pending
      }
    } else {
      Poll::Ready(None)
    }
  }
}

pub struct DefaultSingleValueGroupChannel;

impl<T: Send + Sync + 'static> ChannelLike<T> for DefaultSingleValueGroupChannel {
  type Message = Vec<T>;

  type Sender = GroupSingleValueSender<T>;

  type Receiver = GroupSingleValueReceiver<T>;

  fn build(&mut self) -> (Self::Sender, Self::Receiver) {
    todo!()
  }

  fn send(sender: &Self::Sender, message: T) -> bool {
    todo!()
  }

  fn is_closed(sender: &Self::Sender) -> bool {
    todo!()
  }
}
