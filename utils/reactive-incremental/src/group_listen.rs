use std::{
  sync::{Arc, Mutex, Weak},
  task::Waker,
};

use fast_hash_collection::*;

use crate::*;

impl<T: IncrementalBase> IncrementalSignalStorage<T> {
  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> impl Stream<Item = N> + Unpin
  where
    U: Send + Sync + 'static,
    C: ChannelLike<CollectionDelta<ItemAllocIndex<T>, U>, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    {
      let data = self.inner.data.write();

      for (index, data) in data.iter() {
        mapper(MaybeDeltaRef::All(&data.data), &|mapped| {
          C::send(&sender, CollectionDelta::Delta(index.into(), mapped));
        })
      }
    }

    // could we try another way to workaround this??
    let s: &'static Self = unsafe { std::mem::transmute(self) };

    let remove_token = s.on(move |v| {
      match v {
        StorageGroupChange::Create { index, data } => mapper(MaybeDeltaRef::All(data), &|mapped| {
          C::send(&sender, CollectionDelta::Delta(*index, mapped));
        }),
        StorageGroupChange::Mutate { index, delta } => {
          mapper(MaybeDeltaRef::Delta(delta), &|mapped| {
            C::send(&sender, CollectionDelta::Delta(*index, mapped));
          });
        }
        StorageGroupChange::Drop { index } => {
          C::send(&sender, CollectionDelta::Remove(*index));
        }
      }

      C::is_closed(&sender)
    });

    let dropper = EventSourceDropper::new(remove_token, self.inner.group_watchers.make_weak());
    DropperAttachedStream::new(dropper, receiver)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = GroupSingleValueChangeBuffer<T, U>> + Unpin
  where
    U: Send + Sync + Clone + 'static,
  {
    self.listen_by::<_, _, _>(mapper, &mut DefaultSingleValueGroupChannel)
  }

  pub fn single_listen_by_into_reactive_collection<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static + Clone,
  ) -> impl ReactiveCollection<ItemAllocIndex<T>, U>
  where
    U: Send + Sync + Clone + 'static,
    U: IncrementalBase<Delta = U>,
  {
    let mapper_c = Box::new(mapper.clone());
    ReactiveCollectionForSingleValue::<T, _, U> {
      inner: self.single_listen_by(mapper),
      original: self.clone(),
      mapper: Mutex::new(mapper_c),
    }
  }
}

pub struct GroupSingleValueChangeBuffer<K, T> {
  inner: FastHashMap<ItemAllocIndex<K>, GroupSingleValueState<T>>,
}

impl<K, T> Default for GroupSingleValueChangeBuffer<K, T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<K, T> IntoIterator for GroupSingleValueChangeBuffer<K, T> {
  type Item = CollectionDelta<ItemAllocIndex<K>, T>;
  type IntoIter = impl Iterator<Item = Self::Item>;

  fn into_iter(self) -> Self::IntoIter {
    self.inner.into_iter().flat_map(|(id, state)| {
      let mut expand = smallvec::SmallVec::<[CollectionDelta<ItemAllocIndex<K>, T>; 2]>::new();
      match state {
        GroupSingleValueState::Next(v) => expand.push(CollectionDelta::Delta(id, v)),
        GroupSingleValueState::RemoveThenNext(v) => {
          expand.push(CollectionDelta::Remove(id));
          expand.push(CollectionDelta::Delta(id, v));
        }
        GroupSingleValueState::Remove => expand.push(CollectionDelta::Remove(id)),
      }
      expand
    })
  }
}

pub struct GroupSingleValueSender<K, T> {
  inner: Weak<Mutex<(GroupSingleValueChangeBuffer<K, T>, Option<Waker>)>>,
}

impl<K, T> Drop for GroupSingleValueSender<K, T> {
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

pub struct GroupSingleValueReceiver<K, T> {
  inner: Arc<Mutex<(GroupSingleValueChangeBuffer<K, T>, Option<Waker>)>>,
}

struct ReactiveCollectionForSingleValue<T: IncrementalBase, S, U: IncrementalBase> {
  original: IncrementalSignalStorage<T>,
  inner: S,
  mapper: Mutex<Box<dyn FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync>>,
}

impl<T: IncrementalBase, S, U: IncrementalBase> VirtualCollection<ItemAllocIndex<T>, U>
  for ReactiveCollectionForSingleValue<T, S, U>
{
  fn iter_key(&self, _skip_cache: bool) -> impl Iterator<Item = ItemAllocIndex<T>> + '_ {
    let data = self.original.inner.data.read();
    // todo, use unsafe to avoid clone
    let cloned_keys = data.iter().map(|v| v.0.into()).collect::<Vec<_>>();
    cloned_keys.into_iter()
  }
  fn access(&self, _: bool) -> impl Fn(&ItemAllocIndex<T>) -> Option<U> + '_ {
    let data = self.original.inner.data.read();
    move |key| {
      data.try_get(key.index).map(|v| &v.data).map(|v| {
        // this is not good, but i will keep it
        let result: std::cell::RefCell<Option<_>> = Default::default();

        (self.mapper.lock().unwrap())(
          MaybeDeltaRef::All(unsafe { std::mem::transmute(v) }),
          &|mapped| *result.borrow_mut() = Some(mapped),
        );
        let mut r = result.borrow_mut();
        r.take().unwrap()
      })
    }
  }
}

impl<T, S, U> ReactiveCollection<ItemAllocIndex<T>, U> for ReactiveCollectionForSingleValue<T, S, U>
where
  T: IncrementalBase,
  U: IncrementalBase,
  S: Stream<Item = GroupSingleValueChangeBuffer<T, U>> + Unpin + 'static,
{
  type Changes = GroupSingleValueChangeBuffer<T, U>;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self.inner.poll_next_unpin(cx)
  }
}

impl<K, T> Stream for GroupSingleValueReceiver<K, T> {
  type Item = GroupSingleValueChangeBuffer<K, T>;

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

impl<T: Send + Clone + Sync + 'static, K: Send + Sync + 'static>
  ChannelLike<CollectionDelta<ItemAllocIndex<K>, T>> for DefaultSingleValueGroupChannel
{
  type Message = GroupSingleValueChangeBuffer<K, T>;

  type Sender = GroupSingleValueSender<K, T>;

  type Receiver = GroupSingleValueReceiver<K, T>;

  fn build(&mut self) -> (Self::Sender, Self::Receiver) {
    let receiver = GroupSingleValueReceiver {
      inner: Arc::new(Mutex::new((Default::default(), None))),
    };
    let updater = GroupSingleValueSender {
      inner: Arc::downgrade(&receiver.inner),
    };
    (updater, receiver)
  }

  fn send(sender: &Self::Sender, message: CollectionDelta<ItemAllocIndex<K>, T>) -> bool {
    if let Some(inner) = sender.inner.upgrade() {
      let mut inner = inner.lock().unwrap();

      let mut should_remove = None;

      use CollectionDelta as Change;
      use GroupSingleValueState as State;

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
