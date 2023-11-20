use std::sync::{Arc, Weak};

use fast_hash_collection::*;
use futures::task::AtomicWaker;
use parking_lot::RwLock;

use crate::*;

impl<T: IncrementalBase> IncrementalSignalStorage<T> {
  // in mapper, if receive full, should not return none!
  pub fn listen_to_reactive_collection<U>(
    &self,
    mapper: impl Fn(MaybeDeltaRef<T>) -> Option<U> + Copy + Send + Sync + 'static,
  ) -> impl ReactiveCollection<AllocIdx<T>, U>
  where
    U: Clone + Send + Sync + 'static,
  {
    let inner = Arc::new((Default::default(), AtomicWaker::new()));
    let receiver = GroupSingleValueReceiver {
      inner: inner.clone(),
    };
    let sender = GroupSingleValueSender {
      inner: Arc::downgrade(&receiver.inner),
    };

    {
      let data = self.inner.data.write();

      for (index, data) in data.iter() {
        let mapped = mapper(MaybeDeltaRef::All(&data.data)).unwrap();
        let change = SingleValueGroupChange {
          key: index.into(),
          change: GroupSingleValueState::NewInsert(mapped),
        };
        sender.send(change);
      }
    }

    // could we try another way to workaround this??
    let s: &'static Self = unsafe { std::mem::transmute(self) };

    let remove_token = s.on(move |v| {
      match v {
        StorageGroupChange::Create { index, data } => {
          if let Some(mapped) = mapper(MaybeDeltaRef::All(data)) {
            let change = SingleValueGroupChange {
              key: *index,
              change: GroupSingleValueState::NewInsert(mapped),
            };
            sender.send(change);
          }
        }
        StorageGroupChange::Mutate {
          index,
          delta,
          data_before_mutate,
        } => {
          if let Some(mapped) = mapper(MaybeDeltaRef::Delta(delta)) {
            let mapped_before = mapper(MaybeDeltaRef::All(data_before_mutate)).unwrap();
            let change = SingleValueGroupChange {
              key: *index,
              change: GroupSingleValueState::ChangeTo(mapped, mapped_before),
            };
            sender.send(change);
          }
        }
        StorageGroupChange::Drop { index, data } => {
          let mapped = mapper(MaybeDeltaRef::All(data)).unwrap();
          let change = SingleValueGroupChange {
            key: *index,
            change: GroupSingleValueState::Remove(mapped),
          };
          sender.send(change);
        }
      }

      sender.is_closed()
    });

    let dropper = EventSourceDropper::new(remove_token, self.inner.group_watchers.make_weak());
    let source = DropperAttachedStream::new(dropper, receiver);

    let mapper_c = Box::new(mapper);
    ReactiveCollectionForSingleValue::<T, _, U> {
      inner: source,
      original: self.clone(),
      mutations: inner,
      mapper: mapper_c,
    }
  }
}

pub struct GroupSingleValueChangeBuffer<K, T> {
  inner: Arc<RwLock<GroupSingleValueChangeBufferImpl<K, T>>>,
}

pub struct GroupSingleValueChangeBufferImpl<K, T> {
  inner: FastHashMap<AllocIdx<K>, GroupSingleValueState<T>>,
}

impl<K, T> Default for GroupSingleValueChangeBufferImpl<K, T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<K, T> GroupSingleValueChangeBuffer<K, T> {
  fn take(&self) -> Option<GroupSingleValueChangeBufferImpl<K, T>> {
    let mut inner = self.inner.write();
    let inner: &mut GroupSingleValueChangeBufferImpl<K, T> = &mut inner;
    if inner.inner.is_empty() {
      None
    } else {
      std::mem::take(inner).into()
    }
  }
}

impl<K, T: Clone> Clone for GroupSingleValueChangeBuffer<K, T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<K, T> Default for GroupSingleValueChangeBuffer<K, T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<K, T: Clone> IntoIterator for GroupSingleValueChangeBufferImpl<K, T> {
  type Item = CollectionDelta<AllocIdx<K>, T>;
  type IntoIter = impl Iterator<Item = Self::Item> + Clone;

  fn into_iter(self) -> Self::IntoIter {
    let buffer = self
      .inner
      .into_iter()
      .flat_map(|(id, state)| {
        let mut expand = smallvec::SmallVec::<[CollectionDelta<AllocIdx<K>, T>; 2]>::new();
        match state {
          GroupSingleValueState::NewInsert(v) => expand.push(CollectionDelta::Delta(id, v, None)),
          GroupSingleValueState::ChangeTo(v, p) => {
            expand.push(CollectionDelta::Delta(id, v, Some(p)));
          }
          GroupSingleValueState::Remove(p) => expand.push(CollectionDelta::Remove(id, p)),
        }
        expand
      })
      .collect::<Vec<_>>(); // todo, fix hashmap into_iter not impl clone
    buffer.into_iter()
  }
}

pub struct GroupSingleValueSender<K, T> {
  inner: Weak<(GroupSingleValueChangeBuffer<K, T>, AtomicWaker)>,
}

impl<K, T: Clone> GroupSingleValueSender<K, T> {
  fn send(&self, message: SingleValueGroupChange<AllocIdx<K>, T>) -> bool {
    if let Some(inner) = self.inner.upgrade() {
      let mut should_remove = None;

      use GroupSingleValueState as State;

      let key = message.key;
      let mut mutations = inner.0.inner.write();
      mutations
        .inner
        .entry(key)
        .and_modify(|state| {
          *state = match state {
            State::NewInsert(_) => match &message.change {
              State::NewInsert(_new) => unreachable!(),
              State::ChangeTo(new, _old) => State::NewInsert(new.clone()),
              State::Remove(_old) => {
                should_remove = Some(key);
                return;
              }
            },
            State::ChangeTo(_current_new, old) => match &message.change {
              State::NewInsert(_new) => unreachable!(),
              State::ChangeTo(new, _current_new) => State::ChangeTo(new.clone(), old.clone()),
              State::Remove(old) => State::Remove(old.clone()),
            },
            State::Remove(old) => match &message.change {
              State::NewInsert(new) => State::ChangeTo(new.clone(), old.clone()),
              State::ChangeTo(_new, _current_new) => unreachable!(),
              State::Remove(_old) => unreachable!(),
            },
          }
        })
        .or_insert(message.change.clone()); // we should do some check here?

      if let Some(remove) = should_remove {
        mutations.inner.remove(&remove);
      }

      inner.1.wake();
      true
    } else {
      false
    }
  }
  fn is_closed(&self) -> bool {
    self.inner.upgrade().is_none()
  }
}

impl<K, T> Drop for GroupSingleValueSender<K, T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.1.wake()
    }
  }
}

#[derive(Clone)]
pub enum GroupSingleValueState<T> {
  NewInsert(T),
  // new, old
  ChangeTo(T, T),
  Remove(T),
}

pub struct SingleValueGroupChange<K, T> {
  key: K,
  change: GroupSingleValueState<T>,
}

pub struct GroupSingleValueReceiver<K, T> {
  inner: Arc<(GroupSingleValueChangeBuffer<K, T>, AtomicWaker)>,
}

struct ReactiveCollectionForSingleValue<T: IncrementalBase, S, U> {
  original: IncrementalSignalStorage<T>,
  inner: S,
  mutations: Arc<(GroupSingleValueChangeBuffer<T, U>, AtomicWaker)>, // this is a little messy
  mapper: Box<dyn Fn(MaybeDeltaRef<T>) -> Option<U> + Send + Sync>,
}

impl<T: IncrementalBase, S, U> VirtualCollection<AllocIdx<T>, U>
  for ReactiveCollectionForSingleValue<T, S, U>
{
  fn iter_key(&self) -> impl Iterator<Item = AllocIdx<T>> + '_ {
    let data = self.original.inner.data.read_recursive();
    let mutations = self.mutations.0.inner.read_recursive();
    // todo, use unsafe to avoid clone
    let cloned_keys = data
      .iter()
      .map(|v| v.0.into())
      .filter(|v| !mutations.inner.contains_key(v))
      .chain(mutations.inner.keys().cloned()) // mutations contains removed but not polled key
      .collect::<Vec<_>>();

    cloned_keys.into_iter()
  }
  fn access(&self) -> impl Fn(&AllocIdx<T>) -> Option<U> + '_ {
    let data = self.original.inner.data.read_recursive();
    let mutations = self.mutations.0.inner.read_recursive();
    move |key| {
      if let Some(m) = mutations.inner.get(key) {
        match m {
          GroupSingleValueState::NewInsert(_) => None,
          GroupSingleValueState::ChangeTo(_, old) => {
            (self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(old) }))
              .unwrap()
              .into()
          }
          GroupSingleValueState::Remove(old) => {
            (self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(old) }))
              .unwrap()
              .into()
          }
        }
      } else {
        data
          .try_get(key.index)
          .map(|v| &v.data)
          .map(|v| (self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(v) })).unwrap())
      }
    }
  }
}

impl<T, S, U> ReactiveCollection<AllocIdx<T>, U> for ReactiveCollectionForSingleValue<T, S, U>
where
  T: IncrementalBase,
  U: Clone + 'static,
  S: Stream<Item = GroupSingleValueChangeBufferImpl<T, U>> + Unpin + 'static,
{
  type Changes = impl Iterator<Item = CollectionDelta<AllocIdx<T>, U>> + Clone;

  fn poll_changes(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Changes>> {
    self
      .inner
      .poll_next_unpin(cx)
      .map(|v| v.map(|v| v.into_iter()))
  }
}

impl<K, T> Stream for GroupSingleValueReceiver<K, T> {
  type Item = GroupSingleValueChangeBufferImpl<K, T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    // check is_some first to avoid unnecessary move
    if let Some(change) = self.inner.0.take() {
      Poll::Ready(Some(change))
      // check if sender has dropped
    } else if Arc::weak_count(&self.inner) == 0 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}
