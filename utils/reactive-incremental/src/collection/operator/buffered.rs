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

    match self.inner.poll_changes(cx) {
      CPoll::Ready(delta) => match delta {
        Poll::Ready(delta) => {
          if buffered.is_empty() {
            // if previous is not buffered, we just emit the upstream and avoid materialize
            CPoll::Ready(Poll::Ready(delta))
          } else {
            merge_into_hashmap(&mut buffered, delta.iter_key_value());
            CPoll::Ready(Poll::Ready(Arc::new(buffered).into_boxed()))
          }
        }
        Poll::Pending => {
          return CPoll::Ready(if buffered.is_empty() {
            Poll::Pending
          } else {
            // if previous is buffered, we should emit the buffered change
            Poll::Ready(Arc::new(buffered).into_boxed())
          });
        }
      },
      CPoll::Blocked => {
        *self.buffered.write() = buffered.into();
        CPoll::Blocked
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
