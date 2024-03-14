use std::sync::{Arc, Weak};

use fast_hash_collection::*;
use futures::task::AtomicWaker;
use parking_lot::RwLock;
use storage::IndexReusedVec;

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

#[derive(Clone, Copy, Debug)]
pub struct AnyChanging;
impl PartialEq for AnyChanging {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl<T: IncrementalBase> IncrementalSignalStorage<T> {
  // todo, share all set for same T
  pub fn listen_all_instance_changed_set(
    &self,
  ) -> impl ReactiveCollection<AllocIdx<T>, AnyChanging> {
    self.listen_to_reactive_collection(|_| ChangeReaction::Care(Some(AnyChanging)))
  }

  // in mapper, if receive full, should not return not care!
  pub fn listen_to_reactive_collection<U>(
    &self,
    mapper: impl Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Copy + Send + Sync + 'static,
  ) -> impl ReactiveCollection<AllocIdx<T>, U>
  where
    U: CValue,
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
          sender.send(index.into(), ValueChange::Delta(init, None));
        }
      }
    }

    // could we try another way to workaround this??
    let s: &'static Self = unsafe { std::mem::transmute(self) };

    let remove_token = s.on(move |v| {
      match v {
        StorageGroupChange::Create { index, data } => {
          if let Some(mapped) = mapper(MaybeDeltaRef::All(data)).expect_care() {
            sender.send(*index, ValueChange::Delta(mapped, None));
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
              (None, Some(pre)) => ValueChange::Remove(pre).into(),
              (Some(new), None) => ValueChange::Delta(new, None).into(),
              (Some(new), Some(pre)) => ValueChange::Delta(new, Some(pre)).into(),
            };

            if let Some(change) = change {
              sender.send(*index, change);
            }
          }
        }
        StorageGroupChange::Drop { index, data } => {
          if let Some(mapped) = mapper(MaybeDeltaRef::All(data)).expect_care() {
            sender.send(*index, ValueChange::Remove(mapped));
          }
        }
      }

      sender.is_closed()
    });

    let dropper = EventSourceDropper::new(remove_token, self.inner.group_watchers.make_weak());
    let source = DropperAttachedStream::new(dropper, receiver);

    let mapper_c = Arc::new(mapper);
    ReactiveCollectionFromGroupMutation::<T, _, U> {
      inner: RwLock::new(source),
      original: self.clone(),
      _mutations: inner,
      mapper: mapper_c,
    }
  }
}

pub struct GroupMutationSender<K, T> {
  inner: Weak<(
    RwLock<FastHashMap<AllocIdx<K>, ValueChange<T>>>,
    AtomicWaker,
  )>,
}

impl<K, T: CValue> GroupMutationSender<K, T> {
  fn send(&self, idx: AllocIdx<K>, change: ValueChange<T>) -> bool {
    if let Some(inner) = self.inner.upgrade() {
      let mut mutations = inner.0.write();
      if let Some(old_change) = mutations.get_mut(&idx) {
        if !old_change.merge(&change) {
          mutations.remove(&idx);
        }
      } else {
        mutations.insert(idx, change);
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

pub struct GroupMutationReceiver<K, T> {
  inner: Arc<(
    RwLock<FastHashMap<AllocIdx<K>, ValueChange<T>>>,
    AtomicWaker,
  )>,
}

struct ReactiveCollectionFromGroupMutation<T: IncrementalBase, S, U> {
  original: IncrementalSignalStorage<T>,
  inner: RwLock<S>,
  _mutations: Arc<(
    RwLock<FastHashMap<AllocIdx<T>, ValueChange<U>>>,
    AtomicWaker,
  )>,
  mapper: Arc<dyn Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Send + Sync>,
}

struct ReactiveCollectionFromGroupMutationCurrentView<T: IncrementalBase, U> {
  data: LockReadGuardHolder<IndexReusedVec<SignalItem<T>>>,
  mapper: Arc<dyn Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Send + Sync>,
}

impl<T: IncrementalBase, U> Clone for ReactiveCollectionFromGroupMutationCurrentView<T, U> {
  fn clone(&self) -> Self {
    Self {
      data: self.data.clone(),
      mapper: self.mapper.clone(),
    }
  }
}

impl<T: IncrementalBase, U: CValue> VirtualCollection<AllocIdx<T>, U>
  for ReactiveCollectionFromGroupMutationCurrentView<T, U>
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (AllocIdx<T>, U)> + '_> {
    Box::new(
      self
        .data
        .iter()
        .map(|(k, v)| (AllocIdx::from(k), &v.data))
        .filter_map(|(k, v)| {
          ((self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(v) })))
            .flatten()
            .map(|v| (k, v))
        }),
    )
  }

  fn access(&self, key: &AllocIdx<T>) -> Option<U> {
    self
      .data
      .try_get(key.index)
      .map(|v| &v.data)
      .and_then(|v| (self.mapper)(MaybeDeltaRef::All(unsafe { std::mem::transmute(v) })).flatten())
  }
}

impl<T, S, U> ReactiveCollection<AllocIdx<T>, U> for ReactiveCollectionFromGroupMutation<T, S, U>
where
  T: IncrementalBase,
  U: CValue,
  S: Stream<Item = FastHashMap<AllocIdx<T>, ValueChange<U>>> + Unpin + Send + Sync + 'static,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<AllocIdx<T>, U> {
    match self.inner.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => {
        if r.is_empty() {
          Poll::Pending
        } else {
          Poll::Ready(Box::new(r) as Box<dyn VirtualCollection<AllocIdx<T>, ValueChange<U>>>)
        }
      }
      _ => Poll::Pending,
    }
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // here are we not suppose to shrink the storage
  }

  fn access(&self) -> PollCollectionCurrent<AllocIdx<T>, U> {
    let view = ReactiveCollectionFromGroupMutationCurrentView {
      data: self.original.inner.data.make_read_holder(),
      mapper: self.mapper.clone(),
    };

    Box::new(view)
  }
}

impl<K, T> Stream for GroupMutationReceiver<K, T> {
  type Item = FastHashMap<AllocIdx<K>, ValueChange<T>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut FastHashMap<AllocIdx<K>, ValueChange<T>> = &mut changes;

    let changes = std::mem::take(changes);
    if !changes.is_empty() {
      Poll::Ready(Some(changes))
      // check if sender has dropped
    } else if Arc::weak_count(&self.inner) == 0 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}
