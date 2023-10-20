use std::{
  sync::{Arc, Mutex, Weak},
  task::Waker,
};

use fast_hash_collection::*;

use crate::*;

impl<T: IncrementalBase + Clone> IncrementalSignalStorage<T> {
  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(&StorageGroupChange<T>, &dyn Fn(VirtualKVCollectionDelta<u32, U>))
      + Send
      + Sync
      + 'static,
    channel_builder: &mut C,
  ) -> impl Stream<Item = N> + Unpin
  where
    U: Send + Sync + 'static,
    C: ChannelLike<VirtualKVCollectionDelta<u32, U>, Message = N>,
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

    // could we try another way to workaround this??
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

  pub fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(&StorageGroupChange<T>, &dyn Fn(VirtualKVCollectionDelta<u32, U>))
      + Send
      + Sync
      + 'static,
  ) -> impl Stream<Item = GroupSingleValueChangeBuffer<U>> + Unpin
  where
    U: Send + Sync + Clone + 'static,
    U: IncrementalBase<Delta = U>,
  {
    self.listen_by::<_, _, _>(mapper, &mut DefaultSingleValueGroupChannel)
  }

  pub fn single_listen_by_into_reactive_collection<U>(
    &self,
    mapper: impl FnMut(&StorageGroupChange<T>, &dyn Fn(VirtualKVCollectionDelta<u32, U>))
      + Send
      + Sync
      + 'static
      + Clone,
  ) -> impl ReactiveKVCollection<u32, U>
  where
    U: Send + Sync + Clone + 'static,
    U: IncrementalBase<Delta = U>,
  {
    let mapper_c = Box::new(mapper.clone());
    ReactiveKVCollectionForSingleValue::<T, _, U> {
      inner: self.single_listen_by(mapper),
      original: self.clone(),
      mapper: Mutex::new(mapper_c),
    }
  }
}

pub struct GroupSingleValueChangeBuffer<T> {
  inner: FastHashMap<u32, GroupSingleValueState<T>>,
}

impl<T> Default for GroupSingleValueChangeBuffer<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<T> IntoIterator for GroupSingleValueChangeBuffer<T> {
  type Item = VirtualKVCollectionDelta<u32, T>;
  type IntoIter = impl Iterator<Item = Self::Item>;

  fn into_iter(self) -> Self::IntoIter {
    self.inner.into_iter().flat_map(|(id, state)| {
      let mut expand = smallvec::SmallVec::<[VirtualKVCollectionDelta<u32, T>; 2]>::new();
      match state {
        GroupSingleValueState::Next(v) => expand.push(VirtualKVCollectionDelta::Delta(id, v)),
        GroupSingleValueState::RemoveThenNext(v) => {
          expand.push(VirtualKVCollectionDelta::Remove(id));
          expand.push(VirtualKVCollectionDelta::Delta(id, v));
        }
        GroupSingleValueState::Remove => expand.push(VirtualKVCollectionDelta::Remove(id)),
      }
      expand
    })
  }
}

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

struct ReactiveKVCollectionForSingleValue<T: IncrementalBase, S, U: IncrementalBase> {
  original: IncrementalSignalStorage<T>,
  inner: S,
  mapper: Mutex<
    Box<dyn FnMut(&StorageGroupChange<T>, &dyn Fn(VirtualKVCollectionDelta<u32, U>)) + Send + Sync>,
  >,
}

impl<T: IncrementalBase, S, U: IncrementalBase> VirtualKVCollection<u32, U>
  for ReactiveKVCollectionForSingleValue<T, S, U>
{
  fn access(&self, getter: impl FnOnce(&dyn Fn(u32) -> Option<U>)) {
    let data = self.original.inner.data.read();
    getter(&|key| {
      data.try_get(key).map(|v| &v.data).map(|v| {
        // this is not good, but i will keep it
        let result: std::cell::RefCell<Option<_>> = Default::default();
        (self.mapper.lock().unwrap())(
          &StorageGroupChange::Create {
            data: unsafe { std::mem::transmute(v) },
            index: key,
          },
          &|mapped| {
            if let VirtualKVCollectionDelta::Delta(_, v) = mapped {
              *result.borrow_mut() = Some(v)
            }
          },
        );
        let mut r = result.borrow_mut();
        r.take().unwrap()
      })
    })
  }
}

impl<T, S, U> Stream for ReactiveKVCollectionForSingleValue<T, S, U>
where
  T: IncrementalBase,
  U: IncrementalBase,
  S: Stream<Item = GroupSingleValueChangeBuffer<U>> + Unpin,
{
  type Item = GroupSingleValueChangeBuffer<U>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.inner.poll_next_unpin(cx)
  }
}

impl<T> Stream for GroupSingleValueReceiver<T> {
  type Item = GroupSingleValueChangeBuffer<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    if let Ok(mut inner) = self.inner.lock() {
      inner.1 = cx.waker().clone().into();
      // check is_some first to avoid unnecessary move
      if !inner.0.inner.is_empty() {
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
        .inner
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
        inner.0.inner.remove(remove);
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
