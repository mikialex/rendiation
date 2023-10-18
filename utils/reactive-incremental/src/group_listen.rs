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

type GroupSingleValueChangeBuffer<T> = FastHashMap<u32, GroupSingleValueState<T>>;

pub struct GroupSingleValueSender<T> {
  inner: Weak<Mutex<(GroupSingleValueChangeBuffer<T>, Option<Waker>)>>,
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

pub enum GroupSingleValueState<T> {
  Next(T),
  RemoveThenNext(T),
  Remove,
}

pub struct GroupSingleValueReceiver<T> {
  inner: Arc<Mutex<(GroupSingleValueChangeBuffer<T>, Option<Waker>)>>,
}

impl<T> Stream for GroupSingleValueReceiver<T> {
  type Item = GroupSingleValueChangeBuffer<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    if let Ok(mut inner) = self.inner.lock() {
      inner.1 = cx.waker().clone().into();
      // check is_some first to avoid unnecessary move
      if !inner.0.is_empty() {
        let value = std::mem::take(&mut inner.0);
        Poll::Ready(Some(value))
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

impl<T: Send + Clone + Sync + 'static> ChannelLike<VirtualKVCollectionDelta<u32, T>>
  for DefaultSingleValueGroupChannel
{
  type Message = GroupSingleValueChangeBuffer<T>;

  type Sender = GroupSingleValueSender<T>;

  type Receiver = GroupSingleValueReceiver<T>;

  fn build(&mut self) -> (Self::Sender, Self::Receiver) {
    let receiver = GroupSingleValueReceiver {
      inner: Arc::new(Mutex::new((Default::default(), None))),
    };
    let updater = GroupSingleValueSender {
      inner: Arc::downgrade(&receiver.inner),
    };
    (updater, receiver)
  }

  fn send(sender: &Self::Sender, message: VirtualKVCollectionDelta<u32, T>) -> bool {
    if let Some(inner) = sender.inner.upgrade() {
      let mut inner = inner.lock().unwrap();

      let mut should_remove = None;

      use GroupSingleValueState as State;
      use VirtualKVCollectionDelta as Change;

      inner
        .0
        .entry(*message.key())
        .and_modify(|state| {
          *state = match state {
            State::Next(_) => match &message {
              Change::Delta(_, v) => State::Next(v.clone()),
              Change::Remove(k) => {
                should_remove = Some(k);
                return;
              }
            },
            State::RemoveThenNext(_) => match &message {
              Change::Delta(_, v) => State::RemoveThenNext(v.clone()),
              Change::Remove(_) => State::Remove,
            },
            State::Remove => match &message {
              Change::Delta(_, v) => State::RemoveThenNext(v.clone()),
              Change::Remove(_) => unreachable!(),
            },
          }
        })
        .or_insert_with(|| match &message {
          Change::Delta(_, v) => State::Next(v.clone()),
          Change::Remove(_) => State::Remove,
        });

      if let Some(remove) = should_remove {
        inner.0.remove(remove);
      }

      if let Some(waker) = &inner.1 {
        waker.wake_by_ref()
      }
      true
    } else {
      false
    }
  }

  fn is_closed(sender: &Self::Sender) -> bool {
    sender.inner.upgrade().is_none()
  }
}
