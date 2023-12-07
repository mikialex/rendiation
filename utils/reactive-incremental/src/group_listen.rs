use std::sync::{Arc, Weak};

use fast_hash_collection::*;
use futures::task::AtomicWaker;
use parking_lot::RwLock;

use crate::*;

pub enum ChangeReaction<T> {
  Care(Option<T>),
  NotCare,
}

impl<T> ChangeReaction<T> {
  pub fn cared_then<T2>(self, f: impl FnOnce(T) -> Option<T2>) -> ChangeReaction<T2> {
    match self {
      ChangeReaction::Care(Some(v)) => ChangeReaction::Care(f(v)),
      ChangeReaction::Care(None) => ChangeReaction::Care(None),
      ChangeReaction::NotCare => ChangeReaction::NotCare,
    }
  }
  pub fn cared_map<T2>(self, f: impl FnOnce(T) -> T2) -> ChangeReaction<T2> {
    match self {
      ChangeReaction::Care(Some(v)) => ChangeReaction::Care(Some(f(v))),
      ChangeReaction::Care(None) => ChangeReaction::Care(None),
      ChangeReaction::NotCare => ChangeReaction::NotCare,
    }
  }
  pub fn expect_care(self) -> Option<T> {
    match self {
      ChangeReaction::Care(v) => v,
      _ => unreachable!("expect cared result"),
    }
  }
  pub fn flatten(self) -> Option<T> {
    match self {
      ChangeReaction::Care(Some(v)) => Some(v),
      _ => None,
    }
  }
}

impl<T: IncrementalBase> IncrementalSignalStorage<T> {
  // in mapper, if receive full, should not return not care!
  pub fn listen_to_reactive_collection<U>(
    &self,
    mapper: impl Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Copy + Send + Sync + 'static,
  ) -> impl ReactiveCollectionWithPrevious<AllocIdx<T>, U>
  where
    U: Clone + Send + Sync + 'static,
  {
    let inner = Arc::new((Default::default(), AtomicWaker::new()));
    let receiver = GroupMutationReceiver {
      inner: inner.clone(),
    };
    let sender = GroupMutationSender {
      inner: Arc::downgrade(&receiver.inner),
    };

    {
      let data = self.inner.data.write();

      for (index, data) in data.iter() {
        if let Some(init) = mapper(MaybeDeltaRef::All(&data.data)).expect_care() {
          let change = KeyedMutationState {
            key: index.into(),
            change: MutationState::NewInsert(init),
          };
          sender.send(change);
        }
      }
    }

    // could we try another way to workaround this??
    let s: &'static Self = unsafe { std::mem::transmute(self) };

    let remove_token = s.on(move |v| {
      match v {
        StorageGroupChange::Create { index, data } => {
          if let Some(mapped) = mapper(MaybeDeltaRef::All(data)).expect_care() {
            let change = KeyedMutationState {
              key: *index,
              change: MutationState::NewInsert(mapped),
            };
            sender.send(change);
          }
        }
        StorageGroupChange::Mutate {
          index,
          delta,
          data_before_mutate,
        } => {
          if let ChangeReaction::Care(mapped) = mapper(MaybeDeltaRef::Delta(delta)) {
            let mapped_before = mapper(MaybeDeltaRef::All(data_before_mutate)).expect_care();

            let change = match (mapped, mapped_before) {
              (None, None) => None,
              (None, Some(pre)) => MutationState::Remove(pre).into(),
              (Some(new), None) => MutationState::NewInsert(new).into(),
              (Some(new), Some(pre)) => MutationState::ChangeTo(new, pre).into(),
            };

            if let Some(change) = change {
              sender.send(KeyedMutationState {
                key: *index,
                change,
              });
            }
          }
        }
        StorageGroupChange::Drop { index, data } => {
          if let Some(mapped) = mapper(MaybeDeltaRef::All(data)).expect_care() {
            let change = KeyedMutationState {
              key: *index,
              change: MutationState::Remove(mapped),
            };
            sender.send(change);
          }
        }
      }

      sender.is_closed()
    });

    let dropper = EventSourceDropper::new(remove_token, self.inner.group_watchers.make_weak());
    let source = DropperAttachedStream::new(dropper, receiver);

    let mapper_c = Box::new(mapper);
    ReactiveCollectionFromGroupMutation::<T, _, U> {
      inner: source,
      original: self.clone(),
      mutations: inner,
      mapper: mapper_c,
    }
  }
}

pub struct MutationFolder<K, T> {
  inner: FastHashMap<AllocIdx<K>, MutationState<T>>,
}

impl<K, T> Default for MutationFolder<K, T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<K, T> MutationFolder<K, T> {
  fn take(&mut self) -> Option<MutationFolder<K, T>> {
    if self.inner.is_empty() {
      None
    } else {
      std::mem::take(self).into()
    }
  }
}

impl<K, T: Clone> Clone for MutationFolder<K, T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct GroupMutationSender<K, T> {
  inner: Weak<(RwLock<MutationFolder<K, T>>, AtomicWaker)>,
}

impl<K, T: Clone> GroupMutationSender<K, T> {
  fn send(&self, message: KeyedMutationState<AllocIdx<K>, T>) -> bool {
    if let Some(inner) = self.inner.upgrade() {
      let mut should_remove = false;

      use MutationState as State;

      let key = message.key;
      let mut mutations = inner.0.write();
      mutations
        .inner
        .entry(key)
        .and_modify(|state| {
          *state = match state {
            State::NewInsert(_) => match &message.change {
              State::NewInsert(_new) => unreachable!(),
              State::ChangeTo(new, _old) => State::NewInsert(new.clone()),
              State::Remove(_old) => {
                should_remove = true;
                return;
              }
            },
            State::ChangeTo(_current_new, old) => match &message.change {
              State::NewInsert(_new) => unreachable!(),
              State::ChangeTo(new, _current_new) => State::ChangeTo(new.clone(), old.clone()),
              State::Remove(_) => State::Remove(old.clone()),
            },
            State::Remove(old) => match &message.change {
              State::NewInsert(new) => State::ChangeTo(new.clone(), old.clone()),
              State::ChangeTo(_new, _current_new) => unreachable!(),
              State::Remove(_old) => unreachable!(),
            },
          }
        })
        .or_insert(message.change.clone()); // we should do some check here?

      if should_remove {
        mutations.inner.remove(&key);
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

impl<K, T> Drop for GroupMutationSender<K, T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      inner.1.wake()
    }
  }
}

#[derive(Clone)]
pub enum MutationState<T> {
  NewInsert(T),
  // new, old
  ChangeTo(T, T),
  Remove(T),
}

pub struct KeyedMutationState<K, T> {
  key: K,
  change: MutationState<T>,
}

pub struct GroupMutationReceiver<K, T> {
  inner: Arc<(RwLock<MutationFolder<K, T>>, AtomicWaker)>,
}

struct ReactiveCollectionFromGroupMutation<T: IncrementalBase, S, U> {
  original: IncrementalSignalStorage<T>,
  inner: S,
  mutations: Arc<(RwLock<MutationFolder<T, U>>, AtomicWaker)>,
  mapper: Box<dyn Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Send + Sync>,
}

impl<T: IncrementalBase, S: Sync, U: Sync + Send + Clone> VirtualCollection<AllocIdx<T>, U>
  for ReactiveCollectionFromGroupMutation<T, S, U>
{
  fn iter_key(&self) -> impl Iterator<Item = AllocIdx<T>> + '_ {
    let data = self.original.inner.data.read_recursive();
    let mutations = self.mutations.0.read_recursive();
    // todo, use unsafe to avoid clone
    let cloned_keys = data
      .iter()
      .map(|(k, v)| (AllocIdx::from(k), &v.data))
      .filter_map(|(k, v)| {
        (self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(v) }))
          .flatten()
          .map(|_| k)
      })
      .filter(|v| !mutations.inner.contains_key(v))
      .chain(mutations.inner.keys().cloned()) // mutations contains removed but not polled key
      .collect::<Vec<_>>();

    cloned_keys.into_iter()
  }
  fn access(&self) -> impl Fn(&AllocIdx<T>) -> Option<U> + Sync + '_ {
    let data = self.original.inner.data.read_recursive();
    let mutations = self.mutations.0.read_recursive();
    move |key| {
      if let Some(m) = mutations.inner.get(key) {
        match m {
          MutationState::NewInsert(_) => None,
          MutationState::ChangeTo(_, old) => old.clone().into(),
          MutationState::Remove(old) => old.clone().into(),
        }
      } else {
        data.try_get(key.index).map(|v| &v.data).and_then(|v| {
          (self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(v) })).flatten()
        })
      }
    }
  }

  fn try_access(&self) -> Option<Box<dyn Fn(&AllocIdx<T>) -> Option<U> + Sync + '_>> {
    let acc = self.access();
    let boxed = Box::new(acc) as Box<dyn Fn(&AllocIdx<T>) -> Option<U> + Sync + '_>;
    boxed.into()
  }
}

impl<T, S, U> ReactiveCollectionWithPrevious<AllocIdx<T>, U>
  for ReactiveCollectionFromGroupMutation<T, S, U>
where
  T: IncrementalBase,
  U: Clone + Send + Sync + 'static,
  S: Stream<Item = MutationFolder<T, U>> + Unpin + Send + Sync + 'static,
{
  fn poll_changes(
    &mut self,
    cx: &mut Context<'_>,
  ) -> CPoll<CollectionChangesWithPrevious<AllocIdx<T>, U>> {
    match self.inner.poll_next_unpin(cx) {
      Poll::Ready(Some(mutations)) => {
        let r = mutations
          .inner
          .into_iter()
          .flat_map(|(id, state)| {
            let mut expand = smallvec::SmallVec::<
              [(AllocIdx<T>, CollectionDeltaWithPrevious<AllocIdx<T>, U>); 2],
            >::new();
            match state {
              MutationState::NewInsert(v) => {
                expand.push((id, CollectionDeltaWithPrevious::Delta(id, v, None)))
              }
              MutationState::ChangeTo(v, p) => {
                expand.push((id, CollectionDeltaWithPrevious::Delta(id, v, Some(p))));
              }
              MutationState::Remove(p) => {
                expand.push((id, CollectionDeltaWithPrevious::Remove(id, p)))
              }
            }
            expand
          })
          .collect();
        CPoll::Ready(r)
      }
      _ => CPoll::Pending,
    }
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // here are we not suppose to shrink the storage
  }
}

impl<K, T> Stream for GroupMutationReceiver<K, T> {
  type Item = MutationFolder<K, T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    // check is_some first to avoid unnecessary move
    if let Some(change) = self.inner.0.write().take() {
      Poll::Ready(Some(change))
      // check if sender has dropped
    } else if Arc::weak_count(&self.inner) == 0 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}
