use fast_hash_collection::*;

use crate::*;

pub trait ChangeMerge {
  /// return if exist after merge
  fn merge(&mut self, new: &Self) -> bool;
}

impl<K, V> ChangeMerge for CollectionDelta<K, V>
where
  K: PartialEq + Clone,
  V: Clone,
{
  fn merge(&mut self, new: &Self) -> bool {
    use CollectionDelta::*;
    if self.key() != new.key() {
      panic!("only same key change could be merge");
    }
    *self = match (self.clone(), new.clone()) {
      // later override earlier
      (Delta(k, _d1), Delta(_, d2)) => Delta(k, d2),
      // later override earlier
      // if init not exist, remove is still allowed to be multiple
      (Delta(k, _d1), Remove(_)) => Remove(k),
      // later override earlier
      (Remove(k), Delta(_, d1)) => Delta(k, d1),
      // remove is allowed to be multiple
      (Remove(k), Remove(_)) => Remove(k),
    };
    true
  }
}

impl<K, V> ChangeMerge for CollectionDeltaWithPrevious<K, V>
where
  K: PartialEq + Clone,
  V: Clone,
{
  fn merge(&mut self, new: &Self) -> bool {
    use CollectionDeltaWithPrevious::*;
    if self.key() != new.key() {
      panic!("only same key change could be merge");
    }
    *self = match (self.clone(), new.clone()) {
      (Delta(k, _d1, p1), Delta(_, d2, _p2)) => {
        // we should check d1 = d2
        Delta(k, d2, p1)
      }
      (Delta(k, _d1, p1), Remove(_, _p2)) => {
        // we should check d1 = d2
        if let Some(p1) = p1 {
          Remove(k, p1)
        } else {
          return false;
        }
      }
      (Remove(k, _), Delta(_, d1, p2)) => {
        assert!(p2.is_none());
        Delta(k, d1, None)
      }
      (Remove(_, _), Remove(_, _)) => {
        unreachable!("same key with double remove is invalid")
      }
    };

    true
  }
}

impl<K, V> ChangeMerge for FastHashMap<K, V>
where
  K: Eq + std::hash::Hash + Clone,
  V: Clone + ChangeMerge,
{
  fn merge(&mut self, new: &Self) -> bool {
    new.iter().for_each(|(k, d)| {
      let key = k.clone();
      if let Some(current) = self.get_mut(&key) {
        if !current.merge(d) {
          self.remove(&key);
        }
      } else {
        self.insert(key, d.clone());
      }
    });
    !self.is_empty()
  }
}

#[derive(Clone)]
pub struct BufferedCollection<T, M> {
  inner: M,
  buffered: Option<T>,
}

impl<T, M> BufferedCollection<T, M> {
  pub fn new(inner: M) -> Self {
    Self {
      inner,
      buffered: None,
    }
  }
}

impl<T, M, K, V> VirtualCollection<K, V> for BufferedCollection<T, M>
where
  M: VirtualCollection<K, V>,
{
  fn iter_key(&self) -> impl Iterator<Item = K> + '_ {
    self.inner.iter_key()
  }

  fn access(&self) -> impl Fn(&K) -> Option<V> + Sync + '_ {
    self.inner.access()
  }
}

impl<M, K, V> ReactiveCollection<K, V> for BufferedCollection<CollectionChanges<K, V>, M>
where
  M: ReactiveCollection<K, V>,
  V: Send + Sync + 'static + Clone,
  K: Send + Sync + 'static + Eq + std::hash::Hash + Clone,
{
  fn poll_changes(&mut self, cx: &mut Context) -> Poll<Option<CollectionChanges<K, V>>> {
    let mut buffered = self.buffered.take().unwrap_or(Default::default());
    while let Poll::Ready(Some(change)) = self.inner.poll_changes(cx) {
      if buffered.is_empty() {
        buffered = change;
      } else {
        buffered.merge(&change);
      }
    }
    if buffered.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(buffered))
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<M, K, V> ReactiveCollectionWithPrevious<K, V>
  for BufferedCollection<CollectionChangesWithPrevious<K, V>, M>
where
  M: ReactiveCollectionWithPrevious<K, V>,
  V: Send + Sync + 'static + Clone,
  K: Send + Sync + 'static + Eq + std::hash::Hash + Clone,
{
  fn poll_changes(
    &mut self,
    cx: &mut Context,
  ) -> Poll<Option<CollectionChangesWithPrevious<K, V>>> {
    let mut buffered = self.buffered.take().unwrap_or(Default::default());
    while let Poll::Ready(Some(change)) = self.inner.poll_changes(cx) {
      if buffered.is_empty() {
        buffered = change;
      } else {
        buffered.merge(&change);
      }
    }
    if buffered.is_empty() {
      Poll::Pending
    } else {
      Poll::Ready(Some(buffered))
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<T, M> BufferedCollection<T, M> {
  pub fn put_back_to_buffered(&mut self, buffered: T) {
    self.buffered = buffered.into();
  }
}
