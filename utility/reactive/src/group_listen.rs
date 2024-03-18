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

pub trait ChangeProcessor<T: IncrementalBase, K, U>: Copy + Send + Sync + 'static {
  fn react_change(
    &self,
    change: (&T::Delta, &T),
    idx: AllocIdx<T>,
    callback: &dyn Fn(K, ValueChange<U>),
  );
  fn create_iter(&self, v: &T, idx: AllocIdx<T>) -> impl Iterator<Item = (K, U)>;
  fn access(&self, v: &T, k: &K) -> Option<U>;
}

#[derive(Clone, Copy)]
struct SimpleProcessor<F> {
  f: F,
}

impl<F, T, U> ChangeProcessor<T, AllocIdx<T>, U> for SimpleProcessor<F>
where
  F: Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Copy + Send + Sync + 'static,
  T: IncrementalBase,
{
  fn react_change(
    &self,
    change: (&<T as IncrementalBase>::Delta, &T),
    index: AllocIdx<T>,
    callback: &dyn Fn(AllocIdx<T>, ValueChange<U>),
  ) {
    if let ChangeReaction::Care(mapped) = (self.f)(MaybeDeltaRef::Delta(change.0)) {
      let mapped_before = (self.f)(MaybeDeltaRef::All(change.1)).expect_care();

      let change = match (mapped, mapped_before) {
        (None, None) => None,
        (None, Some(pre)) => ValueChange::Remove(pre).into(),
        (Some(new), None) => ValueChange::Delta(new, None).into(),
        (Some(new), Some(pre)) => ValueChange::Delta(new, Some(pre)).into(),
      };

      if let Some(change) = change {
        callback(index, change);
      }
    }
  }

  fn create_iter(&self, v: &T, idx: AllocIdx<T>) -> impl Iterator<Item = (AllocIdx<T>, U)> {
    self.access(v, &idx).map(|v| (idx, v)).into_iter()
  }

  fn access(&self, v: &T, _k: &AllocIdx<T>) -> Option<U> {
    (self.f)(MaybeDeltaRef::All(v)).expect_care()
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
  pub fn listen_to_reactive_collection<U: CValue>(
    &self,
    logic: impl Fn(MaybeDeltaRef<T>) -> ChangeReaction<U> + Copy + Send + Sync + 'static,
  ) -> impl ReactiveCollection<AllocIdx<T>, U> {
    self.listen_to_reactive_collection_custom(SimpleProcessor { f: logic })
  }

  // in mapper, if receive full, should not return not care!
  pub fn listen_to_reactive_collection_custom<K, U, P: ChangeProcessor<T, K, U>>(
    &self,
    watcher: P,
  ) -> impl ReactiveCollection<K, U>
  where
    U: CValue,
    K: CKey + LinearIdentified,
  {
    let (sender, receiver) = group_mutation();

    {
      let data = self.inner.data.write();
      for (index, data) in data.iter() {
        for (k, v) in watcher.create_iter(&data.data, index.into()) {
          sender.send(k, ValueChange::Delta(v, None));
        }
      }
    }

    let dropper = self.on(move |v| {
      match v {
        StorageGroupChange::Create { index, data } => {
          for (k, v) in watcher.create_iter(data, *index) {
            sender.send(k, ValueChange::Delta(v, None));
          }
        }
        StorageGroupChange::Mutate {
          index,
          delta,
          data_before_mutate,
        } => watcher.react_change((delta, data_before_mutate), *index, &|k, v| {
          sender.send(k, v);
        }),
        StorageGroupChange::Drop { index, data } => {
          for (k, v) in watcher.create_iter(data, *index) {
            sender.send(k, ValueChange::Remove(v));
          }
        }
      }

      sender.is_closed()
    });

    let source = DropperAttachedStream::new(dropper, receiver);

    ReactiveCollectionFromGroupMutation::<T, _, P> {
      inner: RwLock::new(source),
      original: self.clone(),
      watcher,
    }
  }
}

pub struct GroupMutationSender<K, T> {
  inner: Weak<(RwLock<FastHashMap<K, ValueChange<T>>>, AtomicWaker)>,
}

impl<K: CKey, T: CValue> GroupMutationSender<K, T> {
  pub fn send(&self, idx: K, change: ValueChange<T>) -> bool {
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
  inner: Arc<(RwLock<FastHashMap<K, ValueChange<T>>>, AtomicWaker)>,
}

pub fn group_mutation<K, T>() -> (GroupMutationSender<K, T>, GroupMutationReceiver<K, T>) {
  let inner = Arc::new((Default::default(), AtomicWaker::new()));
  let receiver = GroupMutationReceiver {
    inner: inner.clone(),
  };
  let sender = GroupMutationSender {
    inner: Arc::downgrade(&receiver.inner),
  };
  (sender, receiver)
}

struct ReactiveCollectionFromGroupMutation<T: IncrementalBase, S, P> {
  original: IncrementalSignalStorage<T>,
  inner: RwLock<S>,
  watcher: P,
}

struct ReactiveCollectionFromGroupMutationCurrentView<T: IncrementalBase, P> {
  data: LockReadGuardHolder<IndexReusedVec<SignalItem<T>>>,
  watcher: P,
}

impl<T: IncrementalBase, P: Clone> Clone for ReactiveCollectionFromGroupMutationCurrentView<T, P> {
  fn clone(&self) -> Self {
    Self {
      data: self.data.clone(),
      watcher: self.watcher.clone(),
    }
  }
}

impl<T, K, U, P> VirtualCollection<K, U> for ReactiveCollectionFromGroupMutationCurrentView<T, P>
where
  T: IncrementalBase,
  K: CKey + LinearIdentified,
  U: CValue,
  P: ChangeProcessor<T, K, U>,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, U)> + '_> {
    Box::new(
      self
        .data
        .iter()
        .map(|(k, v)| (AllocIdx::from(k), &v.data))
        .flat_map(|(k, v)| self.watcher.create_iter(v, k)),
    )
  }

  fn access(&self, key: &K) -> Option<U> {
    self
      .data
      .try_get(key.alloc_index())
      .map(|v| &v.data)
      .and_then(|v| self.watcher.access(v, key))
  }
}

impl<T, S, K, U, P> ReactiveCollection<K, U> for ReactiveCollectionFromGroupMutation<T, S, P>
where
  T: IncrementalBase,
  U: CValue,
  K: CKey + LinearIdentified,
  S: Stream<Item = FastHashMap<K, ValueChange<U>>> + Unpin + Send + Sync + 'static,
  P: ChangeProcessor<T, K, U>,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, U> {
    match self.inner.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => {
        if r.is_empty() {
          Poll::Pending
        } else {
          Poll::Ready(Box::new(r) as Box<dyn VirtualCollection<K, ValueChange<U>>>)
        }
      }
      _ => Poll::Pending,
    }
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // here are we not suppose to shrink the storage
  }

  fn access(&self) -> PollCollectionCurrent<K, U> {
    let view = ReactiveCollectionFromGroupMutationCurrentView {
      data: self.original.inner.data.make_read_holder(),
      watcher: self.watcher,
    };

    Box::new(view)
  }
}

impl<K, T> Stream for GroupMutationReceiver<K, T> {
  type Item = FastHashMap<K, ValueChange<T>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut FastHashMap<K, ValueChange<T>> = &mut changes;

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
