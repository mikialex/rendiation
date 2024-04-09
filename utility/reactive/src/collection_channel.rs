use std::sync::Arc;

use fast_hash_collection::FastHashMap;
use futures::task::AtomicWaker;
use parking_lot::lock_api::RawRwLock;
use parking_lot::RwLock;

use crate::*;

type MutationData<T> = FastHashMap<u32, ValueChange<T>>;

pub fn collective_channel<T>() -> (CollectiveMutationSender<T>, CollectiveMutationReceiver<T>) {
  let inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)> = Default::default();
  let sender = CollectiveMutationSender {
    inner: inner.clone(),
  };
  let receiver = CollectiveMutationReceiver { inner };

  (sender, receiver)
}

pub struct CollectiveMutationSender<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T> Clone for CollectiveMutationSender<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: CValue> CollectiveMutationSender<T> {
  /// # Safety
  ///
  /// this should be called before send
  pub unsafe fn lock(&self) {
    self.inner.0.raw().lock_exclusive()
  }
  /// # Safety
  ///
  /// this should be called after send
  pub unsafe fn unlock(&self) {
    self.inner.1.wake();
    self.inner.0.raw().unlock_exclusive()
  }
  /// # Safety
  ///
  /// this should be called when locked
  pub unsafe fn send(&self, idx: u32, change: ValueChange<T>) {
    let mutations = &mut *self.inner.0.data_ptr();
    if let Some(old_change) = mutations.get_mut(&idx) {
      if !old_change.merge(&change) {
        mutations.remove(&idx);
      }
    } else {
      mutations.insert(idx, change);
    }
  }
  pub fn is_closed(&self) -> bool {
    // self inner is shared between sender and receiver, if not shared anymore it must be
    // receiver not exist anymore, so the channel is closed.
    Arc::strong_count(&self.inner) == 1
  }
}

/// this is not likely to be triggered because component type is not get removed in any time
impl<T> Drop for CollectiveMutationSender<T> {
  fn drop(&mut self) {
    self.inner.1.wake()
  }
}

pub struct CollectiveMutationReceiver<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T: CValue> CollectiveMutationReceiver<T> {
  pub fn poll_impl(
    &self,
    cx: &mut Context,
  ) -> Poll<Option<Box<dyn VirtualCollection<u32, ValueChange<T>>>>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut MutationData<T> = &mut changes;

    let changes = std::mem::take(changes);
    if !changes.is_empty() {
      Poll::Ready(Some(Box::new(changes)))
      // check if the sender has been dropped
    } else if Arc::strong_count(&self.inner) == 1 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}

impl<T: CValue> Stream for CollectiveMutationReceiver<T> {
  type Item = Box<dyn VirtualCollection<u32, ValueChange<T>>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.poll_impl(cx)
  }
}

// this trait could be lift into upper stream
pub trait VirtualCollectionAccess<K, V>: Send + Sync {
  fn access(&self) -> CollectionView<K, V>;
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V> + 'static> VirtualCollectionAccess<K, V>
  for Arc<RwLock<T>>
{
  fn access(&self) -> CollectionView<K, V> {
    Box::new(self.make_read_holder())
  }
}

pub struct ReactiveCollectionFromCollectiveMutation<T> {
  pub full: Box<dyn VirtualCollectionAccess<u32, T>>,
  pub mutation: RwLock<CollectiveMutationReceiver<T>>,
}
impl<T: CValue> ReactiveCollection<u32, T> for ReactiveCollectionFromCollectiveMutation<T> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, T> {
    match self.mutation.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => Poll::Ready(r),
      _ => Poll::Pending,
    }
  }

  fn access(&self) -> PollCollectionCurrent<u32, T> {
    self.full.access()
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // component storage should not shrink here
  }
}
