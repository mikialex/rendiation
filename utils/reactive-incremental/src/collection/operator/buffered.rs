use std::sync::Arc;

use fast_hash_collection::*;

use crate::*;

pub struct BufferedCollection<M, K, V> {
  inner: M,
  buffered: RwLock<Option<FastHashMap<K, ValueChange<V>>>>,
}

impl<M: Clone, K, V> Clone for BufferedCollection<M, K, V> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      buffered: RwLock::new(None),
    }
  }
}

impl<M, K, V> BufferedCollection<M, K, V> {
  pub fn new(inner: M) -> Self {
    Self {
      inner,
      buffered: RwLock::new(None),
    }
  }
}

impl<M, K, V> ReactiveCollection<K, V> for BufferedCollection<M, K, V>
where
  M: ReactiveCollection<K, V>,
  V: CValue,
  K: CKey,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, V> {
    let mut buffered = self.buffered.write().take().unwrap_or(Default::default());
    loop {
      match self.inner.poll_changes(cx) {
        CPoll::Ready(delta) => match delta {
          Poll::Ready(delta) => {
            let delta = delta.materialize_hashmap_maybe_cloned();
            if buffered.is_empty() {
              buffered = delta;
            } else {
              buffered.merge(&delta);
            }
          }
          Poll::Pending => {
            return CPoll::Ready(if buffered.is_empty() {
              Poll::Pending
            } else {
              Poll::Ready(Arc::new(buffered).into_boxed())
            })
          }
        },
        CPoll::Blocked => {
          *self.buffered.write() = buffered.into();
          return CPoll::Blocked;
        }
      }
    }
  }

  fn access(&self) -> PollCollectionCurrent<K, V> {
    self.inner.access()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}

impl<M, K: Clone, V: Clone> BufferedCollection<M, K, V> {
  pub fn put_back_to_buffered(&self, buffered: Arc<FastHashMap<K, ValueChange<V>>>) {
    let buffered = Arc::try_unwrap(buffered).unwrap_or_else(|buffered| buffered.deref().clone());
    *self.buffered.write() = buffered.into();
  }
}

impl<M, K, V> ReactiveOneToManyRelationship<V, K> for BufferedCollection<M, K, V>
where
  M: ReactiveOneToManyRelationship<V, K>,
  K: CKey,
  V: CKey,
{
  fn multi_access(&self) -> CPoll<Box<dyn VirtualMultiCollection<V, K> + '_>> {
    self.inner.multi_access()
  }
}